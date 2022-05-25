use std::sync::Weak;

use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_transform::TransformBundle;
use log::{error, trace, warn};

use pompeii::{mesh::Mesh, PompeiiRenderer};

#[derive(Debug, Component)]
pub struct MeshComponent {
    pub handle: Handle<MeshAsset>,
}

#[derive(Debug, Bundle)]
pub struct MeshBundle {
    pub mesh: MeshComponent,
    #[bundle]
    pub transform: TransformBundle,
}

impl From<Handle<MeshAsset>> for MeshBundle {
    fn from(handle: Handle<MeshAsset>) -> Self {
        Self {
            mesh: MeshComponent { handle },
            transform: TransformBundle::identity(),
        }
    }
}

#[derive(Clone, TypeUuid)]
#[uuid = "c4ff691e-eaee-4369-84da-429838ea6e71"]
pub struct MeshAsset {
    pub(crate) renderer: Weak<PompeiiRenderer>,
    pub(crate) mesh: Mesh,
}

// impl Drop for MeshAsset {
//     fn drop(&mut self) {
//         trace!("Freeing mesh...");
//         if let Some(renderer) = self.renderer.upgrade() {
//             unsafe {
//                 self.mesh.destroy(&renderer);
//             }
//         } else {
//             error!("Trying to free Mesh but the renderer is nowhere to be found");
//         }
//     }
// }
