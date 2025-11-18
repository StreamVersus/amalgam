use vulkan_raw::VkSampleCountFlags;
use crate::both::RenderLoop;
use crate::engine::{create_window, Callbacks, Settings};

pub mod application;
pub mod vulkan;
pub mod engine;
pub mod both;
pub mod pipeline_presets;

fn main() {
    create_window(Settings {
        width: 1200,
        height: 900,
        vsync: false,
        msaa: VkSampleCountFlags::SC_8_BIT,
        callbacks: Callbacks {
            render: RenderLoop::render_loop,
            render_init: RenderLoop::init,
            handle_mouse: RenderLoop::handle_mouse_input,
            key_pressed: RenderLoop::key_pressed,
            key_released: RenderLoop::key_released,
        },
        ..Default::default()
    });
}
