use bitflags::bitflags;
use petgraph::{
    data::Element::Node,
    prelude::{DiGraph, NodeIndex},
};

#[derive(Copy, Clone)]
pub struct Res(NodeIndex);
#[derive(Copy, Clone)]
pub struct Action(NodeIndex);

#[derive(Debug)]
enum NodeId<R, C> {
    Resource(ResourceTy<R>),
    Action(ActionTy<C>),
}

#[derive(Debug)]
enum ResourceTy<Tag> {
    Buffer(Tag),
    Image(Tag),
}

#[derive(Debug)]
enum ActionTy<Tag> {
    External,
    UploadToBuffer(Tag),
    Raster(Tag),
    Compute(Tag),
}

#[derive(Debug)]
enum EdgeAction {
    Read(ReadActionFlags),
    /// If the `bool` is set to `true`, the previous content will be preserved
    Write(WriteActionFlags, bool),
}

bitflags! {
    struct ReadActionFlags: u32 {
        const INPUT_ATTACHMENT = 1 << 0;
        const DEPTH_ATTACHMENT = 1 << 1;
        const SAMPLED = 1 << 2;
        const TRANSFER = 1 << 3;
        const STORAGE = 1 << 4;
    }

    struct WriteActionFlags: u32 {
        const COLOR_ATTACHMENT = 1 << 16;
        const DEPTH_ATTACHMENT = ReadActionFlags::DEPTH_ATTACHMENT.bits;
        const TRANSFER = 1 << 17;
        const STORAGE = 1 << 18;
    }
}

pub struct TaskGraph<R, C> {
    graph: DiGraph<NodeId<R, C>, EdgeAction>,
    external_before_node: NodeIndex,
    external_after_node: NodeIndex,
}

impl<R, C> TaskGraph<R, C> {
    pub fn new() -> Self {
        let mut graph = DiGraph::new();
        let before = graph.add_node(NodeId::Action(ActionTy::External));
        let after = graph.add_node(NodeId::Action(ActionTy::External));

        Self {
            graph,
            external_before_node: before,
            external_after_node: after,
        }
    }

    pub fn register_resource_buffer(&mut self, tag: R) -> Res {
        Res(self
            .graph
            .add_node(NodeId::Resource(ResourceTy::Buffer(tag))))
    }

    pub fn register_resource_image(&mut self, tag: R) -> Res {
        Res(self
            .graph
            .add_node(NodeId::Resource(ResourceTy::Image(tag))))
    }

    pub fn copy_from_host(&mut self, Res(to): Res) {
        self.graph.add_edge(
            self.external_before_node,
            to,
            EdgeAction::Write(WriteActionFlags::TRANSFER, false),
        );
    }

    pub fn copy_to_host(&mut self, Res(from): Res) {
        self.graph.add_edge(
            from,
            self.external_after_node,
            EdgeAction::Read(ReadActionFlags::TRANSFER),
        );
    }

    pub fn create_raster_pass(&mut self, tag: C) -> RasterPassBuilder<R, C> {
        let pass = self.graph.add_node(NodeId::Action(ActionTy::Raster(tag)));
        RasterPassBuilder {
            graph: &mut self.graph,
            pass,
        }
    }

    pub fn create_compute_pass(&mut self, tag: C) -> ComputePassBuilder<R, C> {
        let pass = self.graph.add_node(NodeId::Action(ActionTy::Compute(tag)));
        ComputePassBuilder {
            graph: &mut self.graph,
            pass,
        }
    }
}

pub struct RasterPassBuilder<'graph, R, C> {
    graph: &'graph mut DiGraph<NodeId<R, C>, EdgeAction>,
    pass: NodeIndex,
}

impl<R, C> RasterPassBuilder<'_, R, C> {
    pub fn add_color_attachment(mut self, Res(res): Res) -> Self {
        self.graph.add_edge(
            self.pass,
            res,
            EdgeAction::Write(WriteActionFlags::COLOR_ATTACHMENT, false),
        );
        self
    }

    pub fn add_depth_attachment(mut self, Res(res): Res, preserve: bool) -> Self {
        self.graph.add_edge(
            self.pass,
            res,
            EdgeAction::Write(WriteActionFlags::DEPTH_ATTACHMENT, preserve),
        );
        self
    }

    pub fn add_sampled(mut self, Res(res): Res) -> Self {
        self.graph
            .add_edge(res, self.pass, EdgeAction::Read(ReadActionFlags::SAMPLED));
        self
    }

    pub fn add_bound_buffer(mut self, Res(res): Res) -> Self {
        self.graph
            .add_edge(res, self.pass, EdgeAction::Read(ReadActionFlags::empty()));
        self
    }
}

pub struct ComputePassBuilder<'graph, R, C> {
    graph: &'graph mut DiGraph<NodeId<R, C>, EdgeAction>,
    pass: NodeIndex,
}

impl<R, C> ComputePassBuilder<'_, R, C> {
    pub fn add_input_storage_buffer(mut self, Res(res): Res) -> Self {
        self.graph
            .add_edge(res, self.pass, EdgeAction::Read(ReadActionFlags::STORAGE));
        self
    }

    pub fn add_output_storage_buffer(mut self, Res(res): Res) -> Self {
        self.graph.add_edge(
            self.pass,
            res,
            EdgeAction::Write(WriteActionFlags::STORAGE, false),
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::TaskGraph;
    use petgraph::dot::{Config, Dot};

    #[test]
    fn yolo() {
        let mut graph = TaskGraph::new();

        let model = graph.register_resource_buffer("Model 1");
        let backbuffer = graph.register_resource_image("Backbuffer");
        let depth = graph.register_resource_image("Depth");
        let texture = graph.register_resource_image("Texture 1");

        let in_compute = graph.register_resource_buffer("In compute");
        let out_compute = graph.register_resource_buffer("Out compute");

        graph.copy_from_host(model);
        graph
            .create_raster_pass("Raster")
            .add_bound_buffer(model)
            .add_color_attachment(backbuffer)
            .add_depth_attachment(depth, false)
            .add_sampled(texture);

        graph.copy_from_host(in_compute);
        graph
            .create_compute_pass("Compute")
            .add_input_storage_buffer(in_compute)
            .add_output_storage_buffer(out_compute);
        graph.copy_to_host(out_compute);

        println!("digraph {{");
        println!(" rankdir=LR");
        println!(
            "{:?}",
            Dot::with_config(&graph.graph, &[Config::GraphContentOnly])
        );
        println!("}}");
    }
}
