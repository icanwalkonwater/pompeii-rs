use std::time::{Duration, Instant};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_hierarchy::Children;
use bevy_transform::prelude::{GlobalTransform, Transform};
use log::{debug, info};

use pompeii::PompeiiRenderer;

use crate::{swapchain_recreation as swapchain, swapchain_recreation::RecreateSwapchainEvent};

pub mod gltf_loader;
pub mod mesh;
pub(crate) mod setup;
pub(crate) mod swapchain_recreation;
pub(crate) mod utils;

use crate::mesh::{Mesh, SubMesh};
pub use pompeii;

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
    renderer: NonSend<PompeiiRenderer>,
    mut recreate_swapchain_events: EventWriter<RecreateSwapchainEvent>,
    q_meshes: Query<(&GlobalTransform, &Mesh, &Children)>,
    q_sub_meshes: Query<&SubMesh>,
) {
    let recreate_swapchain = renderer.render_and_present().unwrap();
    if recreate_swapchain {
        recreate_swapchain_events.send(RecreateSwapchainEvent::default())
    }

    for (pos, _, children) in q_meshes.iter() {
        let children: &Children = children;
        let pos: &GlobalTransform = pos;

        debug!("Found mesh at pos with sub meshes:",);
        debug!("{:?}", pos);

        for &child in children.iter() {
            let sub_mesh = q_sub_meshes.get(child).unwrap();
            debug!("- {:?}", sub_mesh);
        }
    }
}
