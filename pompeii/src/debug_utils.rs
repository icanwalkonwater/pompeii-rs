use std::{borrow::Cow, ffi::CStr};

use ash::{vk, vk::Handle};
use log::{log, Level};

use crate::errors::Result;

pub(crate) struct DebugUtils {
    pub(crate) loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl DebugUtils {
    pub(crate) fn new(entry: &ash::Entry, instance: &ash::Instance) -> Result<Self> {
        let loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let messenger = unsafe {
            loader.create_debug_utils_messenger(
                &vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback)),
                None,
            )?
        };

        Ok(Self { loader, messenger })
    }

    pub(crate) fn name_buffer(
        &self,
        device: &ash::Device,
        buffer: vk::Buffer,
        name: &CStr,
    ) -> Result<()> {
        unsafe {
            self.loader.debug_utils_set_object_name(
                device.handle(),
                &vk::DebugUtilsObjectNameInfoEXT::builder()
                    .object_type(vk::ObjectType::BUFFER)
                    .object_handle(buffer.as_raw())
                    .object_name(name),
            )?;
        }
        Ok(())
    }
}

impl DebugUtils {
    pub(crate) unsafe fn destroy(&self) {
        self.loader
            .destroy_debug_utils_messenger(self.messenger, None);
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;

    let message_id_number: i32 = callback_data.message_id_number as i32;
    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };
    let message = if callback_data.p_message.is_null() {
        Cow::from("No message")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    log!(
        severity_to_level(message_severity),
        "[{:?}] [{}/{}] {}",
        message_type,
        message_id_number,
        message_id_name,
        message,
    );

    vk::FALSE
}

fn severity_to_level(severity: vk::DebugUtilsMessageSeverityFlagsEXT) -> Level {
    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => Level::Error,
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => Level::Warn,
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => Level::Info,
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => Level::Trace,
        _ => Level::Trace,
    }
}
