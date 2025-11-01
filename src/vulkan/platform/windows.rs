use vulkan_raw::VkExtensionProperties;

#[inline(always)]
pub fn platform_extensions(supported_extensions: &Vec<VkExtensionProperties>) -> Vec<&'static str> {
    vec!["VK_KHR_win32_surface"]
}
