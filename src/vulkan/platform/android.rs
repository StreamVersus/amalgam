use android_activity::AndroidApp;
use vulkan_raw::VkExtensionProperties;
use crate::both::logic_loop;
use crate::engine::{create_window, Settings};

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    create_window(Settings {
        width: 400,
        height: 600,
        target_fps: 144f64,
        min_fps: 24f64,
        smoothing_factor: 0.7,
        frame: logic_loop,
        activity: Some(app),
        ..Default::default()
    });
}

#[inline(always)]
pub fn platform_extensions(supported_extensions: &Vec<VkExtensionProperties>) -> Vec<&'static str> {
    vec!["VK_KHR_android_surface"]
}