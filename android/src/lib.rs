#![cfg(target_os = "android")]
use amalgam::both::RenderLoop;
use amalgam::engine::{create_window, Callbacks, Settings};
use android_activity::AndroidApp;

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    create_window(Settings {
        width: 400,
        height: 600,
        target_fps: 144f64,
        min_fps: 24f64,
        smoothing_factor: 0.7,
        callbacks: Callbacks {
            render: RenderLoop::render_loop,
            render_init: RenderLoop::init,
            handle_mouse: RenderLoop::handle_mouse_input,
            key_pressed: RenderLoop::key_pressed,
            key_released: RenderLoop::key_released,
        },
        activity: Some(app),
        ..Default::default()
    });
}