use std::collections::HashSet;

const WAYLAND_EXT: &str = "VK_KHR_wayland_surface\0";
const XCB_EXT: &str = "VK_KHR_xcb_surface\0";
const XLIB_EXT: &str = "VK_KHR_xlib_surface\0";
#[inline(always)]
pub fn platform_extensions(supported_extensions: &HashSet<String>) -> Vec<&'static str> {
    if supported_extensions.contains(WAYLAND_EXT) {
        unsafe { IS_WAYLAND = true; }
        vec!["VK_KHR_wayland_surface"]
    } else if supported_extensions.contains(XLIB_EXT) {
        vec!["VK_KHR_xlib_surface"]
    } else if supported_extensions.contains(XCB_EXT) {
        vec!["VK_KHR_xcb_surface"]
    } else {
        panic!("Unsupported window protocol");
    }
}

pub static mut IS_WAYLAND: bool = false;

pub fn is_wayland() -> bool {
    unsafe { IS_WAYLAND }
}