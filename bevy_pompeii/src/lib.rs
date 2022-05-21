use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::prelude::*;
use std::sync::Arc;

pub use pompeii;
use pompeii::PompeiiRenderer;

use crate::{
    gltf_loader::GltfLoader, mesh::MeshAsset, swapchain_recreation as swapchain,
    swapchain_recreation::RecreateSwapchainEvent,
};
use mesh::{free_mesh_on_exit, free_unused_mesh_asset};

pub mod gltf_loader;
pub mod mesh;
pub(crate) mod setup;
pub(crate) mod swapchain_recreation;
pub(crate) mod utils;

#[derive(Clone, Hash, Debug, Eq, PartialEq, StageLabel)]
pub enum RenderStage {
    PreRender,
    Render,
}

#[derive(Default)]
pub struct PompeiiPlugin;

impl Plugin for PompeiiPlugin {
    fn build(&self, app: &mut App) {
        // Events
        app.add_event::<RecreateSwapchainEvent>();

        // Loader will be added later in the setup code
        app.add_asset::<MeshAsset>();
        // Free mesh when asset is dropped
        app.add_system(free_unused_mesh_asset);
        // Free mesh on system exit
        app.add_system_to_stage(CoreStage::Last, free_mesh_on_exit);

        // Renderer will be created in this setup system
        app.add_startup_system(
            setup::setup_renderer_with_window
                .exclusive_system()
                .at_start(),
        );

        // Render systems
        app.add_stage(
            RenderStage::PreRender,
            SystemStage::single_threaded()
                .with_system(swapchain::trigger_recreate_swapchain_system)
                .with_system(
                    swapchain::recreate_swapchain_system
                        .after(swapchain::trigger_recreate_swapchain_system),
                ),
        );
        app.add_stage_after(
            RenderStage::PreRender,
            RenderStage::Render,
            SystemStage::single_threaded()
                .with_system_set(SystemSet::new().with_system(render_system)),
        );
    }
}

fn render_system(
    renderer: ResMut<Arc<PompeiiRenderer>>,
    mut recreate_swapchain_events: EventWriter<RecreateSwapchainEvent>,
) {
    let recreate_swapchain = renderer.render_and_present().unwrap();
    if recreate_swapchain {
        recreate_swapchain_events.send(RecreateSwapchainEvent::default())
    }

    // for (pos, _, children) in q_meshes.iter() {
    //     let children: &Children = children;
    //     let pos: &GlobalTransform = pos;
    //
    //     debug!("Found mesh at pos with sub meshes:",);
    //     debug!("{:?}", pos);
    //
    //     for &child in children.iter() {
    //         let sub_mesh = q_sub_meshes.get(child).unwrap();
    //         debug!("- {:?}", sub_mesh);
    //     }
    // }
}
