use std::collections::HashSet;

#[inline(always)]
pub fn platform_extensions(supported_extensions: &HashSet<String>) -> Vec<&'static str> {
    vec!["VK_KHR_win32_surface"]
}
