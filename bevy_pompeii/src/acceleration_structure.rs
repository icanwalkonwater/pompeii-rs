use bevy_asset::Handle;
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;

use pompeii::acceleration_structure::{Blas, Tlas};

#[derive(Component)]
pub struct BlasComponent {
    pub handle: Handle<BlasAsset>,
}

#[derive(TypeUuid)]
#[uuid = "ba78df52-efcd-46b1-9262-bfe494f34d93"]
pub struct BlasAsset {
    pub(crate) blas: Blas,
}

#[derive(Component)]
pub struct TlasComponent {
    pub handle: Handle<TlasAsset>,
}

#[derive(TypeUuid)]
#[uuid = "12ce7eaa-424a-4b86-bdc5-13a2148798e8"]
pub struct TlasAsset {
    pub(crate) tlas: Tlas,
}
