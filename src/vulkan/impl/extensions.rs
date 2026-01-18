use crate::vulkan::func::Vulkan;
use std::ptr::{null, null_mut};
use vulkan_raw::{vkEnumerateInstanceExtensionProperties, VkExtensionProperties};

impl Vulkan {
    pub fn get_extensions() -> Vec<VkExtensionProperties> {
        let mut extension_count: u32 = 0;
        let mut result;
        unsafe {
            result = vkEnumerateInstanceExtensionProperties(null(), &mut extension_count, null_mut());
        }
        assert!(result.is_ok());
        assert_ne!(extension_count, 0);
        
        let mut extensions : Vec<VkExtensionProperties> = Vec::with_capacity(extension_count as usize);
        let spare = extensions.spare_capacity_mut();
        unsafe {
            result = vkEnumerateInstanceExtensionProperties(null(), &mut extension_count, spare.as_mut_ptr() as *mut VkExtensionProperties);
        }
        assert!(result.is_ok());
        assert_ne!(extension_count, 0);
        
        unsafe {
            extensions.set_len(extension_count as usize);
        }
        extensions
    }
}