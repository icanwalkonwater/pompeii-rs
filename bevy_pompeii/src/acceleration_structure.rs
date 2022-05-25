use std::sync::Weak;

use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use log::{error, trace};

use pompeii::{acceleration_structure::Blas, PompeiiRenderer};

#[derive(Component)]
pub struct BlasComponent {
    pub handle: Handle<BlasAsset>,
}

#[derive(TypeUuid)]
#[uuid = "ba78df52-efcd-46b1-9262-bfe494f34d93"]
pub struct BlasAsset {
    pub(crate) renderer: Weak<PompeiiRenderer>,
    pub(crate) blas: Blas,
}

// impl Drop for BlasAsset {
//     fn drop(&mut self) {
//         trace!("Freeing BLAS...");
//         if let Some(renderer) = self.renderer.upgrade() {
//             unsafe {
//                 self.blas.destroy(&renderer);
//             }
//         } else {
//             error!("Trying to free BLAS but the renderer is nowhere to be found");
//         }
//     }
// }
