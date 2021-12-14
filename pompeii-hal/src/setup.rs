use crate::{errors::Result, PompeiiBackend};
use std::{ffi::CStr, os::raw::c_char};

pub trait PompeiiInitializer: Sized {
    type Backend: PompeiiBackend;

    fn new() -> Self;
    fn with_name(self, name: &str) -> Self;
    fn with_instance_extension(self, name: &'static CStr) -> Self;
    fn build(
        self,
    ) -> Result<
        <<Self as PompeiiInitializer>::Backend as PompeiiBackend>::Error,
        <<Self as PompeiiInitializer>::Backend as PompeiiBackend>::Builder,
    >;
}
