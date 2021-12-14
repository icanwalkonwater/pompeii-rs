use crate::{errors::Result, PompeiiApp, PompeiiBackend};
use std::sync::Arc;

pub trait PompeiiBuilder {
    type Backend: PompeiiBackend;

    fn builder() -> <<Self as PompeiiBuilder>::Backend as PompeiiBackend>::Initializer;
    fn build(
        self,
    ) -> Result<
        <<Self as PompeiiBuilder>::Backend as PompeiiBackend>::Error,
        Arc<PompeiiApp<Self::Backend>>,
    >;
}
