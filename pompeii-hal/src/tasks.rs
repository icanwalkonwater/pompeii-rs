use std::marker::PhantomData;

pub enum TaskType {
    CopyBuffer,
}

impl TaskType {
    pub(crate) fn is_transfer_compatible(&self) -> bool {
        match self {
            Self::CopyBuffer => true,
            _ => false,
        }
    }

    pub(crate) fn is_graphics_compatible(&self) -> bool {
        match self {
            Self::CopyBuffer => true,
            _ => false,
        }
    }

    pub(crate) fn is_compute_compatible(&self) -> bool {
        match self {
            Self::CopyBuffer => true,
            _ => false,
        }
    }
}

pub struct TaskBuilder<'a> {
    _p: PhantomData<&'a ()>,
}
