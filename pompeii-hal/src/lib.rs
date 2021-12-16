use crate::{alloc::PompeiiAllocator, errors::BackendError, tasks::TaskBuilder};
use setup::{builder::PompeiiBuilder, initializer::PompeiiInitializer};

pub mod alloc;
pub mod setup;
pub mod tasks;

pub mod errors {
    use std::error::Error;
    use thiserror::Error;

    pub type Result<B, T> = std::result::Result<T, PompeiiError<B>>;

    pub trait BackendError: Error {}

    #[derive(Error, Debug)]
    pub enum PompeiiError<BACKEND: BackendError> {
        #[error("No compute queue found")]
        NoComputeQueue,
        #[error("No transfer queue found")]
        NoTransferQueue,
        #[error("No physical device picked")]
        NoPhysicalDevicePicked,
        #[error("{0}")]
        BackendError(#[from] BACKEND),
    }
}

pub trait PompeiiBackend {
    type Error: BackendError;
    type Initializer: PompeiiInitializer;
    type Builder: PompeiiBuilder;
    type Allocator: PompeiiAllocator;

    // fn tasker(&self) -> TaskBuilder;
}

pub struct PompeiiApp<B: PompeiiBackend> {
    pub(crate) backend: B,
}

impl<B: PompeiiBackend> From<B> for PompeiiApp<B> {
    fn from(backend: B) -> Self {
        Self { backend }
    }
}
