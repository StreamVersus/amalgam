use crate::application::WINDOW_TITLE;
use crate::both::RenderLoop;
use crate::engine::{Delta, Settings};
use crate::vulkan::func::Vulkan;
use crate::vulkan::r#impl::swapchain::SwapchainInfo;
use std::collections::HashSet;
use vulkan_raw::{VkSurfaceKHR, VkSwapchainKHR};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

#[derive(Default)]
pub struct App {
    pub window: Option<Window>,
    pub delta: Delta,
    pub settings: Settings,
    pub vulkan: Vulkan,
    pub swapchain_info: SwapchainInfo,
    pub render_loop: RenderLoop,
    pub pressed_keys: HashSet<KeyCode>,
    
    pub focused: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(PhysicalSize::new(self.settings.width, self.settings.height))
            .with_visible(true);
        let window = event_loop.create_window(window_attrs).unwrap();

        let surface = self.vulkan.connect_vulkan(&window);
        self.swapchain_info
            .set_width(self.settings.width)
            .set_height(self.settings.height)
            .set_format(self.settings.render_format.clone())
            .set_surface(surface);
        self.vulkan.create_swapchain(&mut self.swapchain_info);

        (self.settings.callbacks.render_init)(&mut self.render_loop, &mut self.vulkan, &mut self.swapchain_info, &mut self.settings);

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.vulkan.destroy_swapchain(self.swapchain_info.swapchain);
                self.swapchain_info.swapchain = VkSwapchainKHR::none();
                
                self.vulkan.destroy_surface(self.swapchain_info.surface);
                self.swapchain_info.surface = VkSurfaceKHR::none();

                self.vulkan.device_wait();
                self.render_loop = RenderLoop::default();
                self.vulkan.finish();

                event_loop.exit();
            },
            WindowEvent::Resized(new_size) => {
                if self.swapchain_info.width != new_size.width || self.swapchain_info.height != new_size.height {
                    self.swapchain_info.set_width(new_size.width).set_height(new_size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                (self.settings.callbacks.render)(&mut self.render_loop, &mut self.vulkan, &mut self.swapchain_info, self.delta.tick());
                self.delta.sleep_till_next_frame();
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Focused(focused) => {
                let window = self.window.as_ref().unwrap();
                if focused {
                    window.set_cursor_visible(false);
                    window.set_cursor_grab(CursorGrabMode::Confined)
                        .unwrap_or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked).unwrap());
                } else {
                    window.set_cursor_visible(true);
                    window.set_cursor_grab(CursorGrabMode::None).unwrap();
                    for key in &self.pressed_keys {
                        (self.settings.callbacks.key_released)(&mut self.render_loop, *key);
                    }
                    self.pressed_keys.clear();
                }
                self.focused = focused;
            }
            WindowEvent::KeyboardInput { device_id: _device_id, event, is_synthetic: _is_synthetic } => {
                match event.physical_key {
                    PhysicalKey::Code(key_code) => {
                        if event.state == ElementState::Pressed {
                            if self.pressed_keys.insert(key_code) {
                                (self.settings.callbacks.key_pressed)(&mut self.render_loop, key_code);
                            }
                        } else {
                            if self.pressed_keys.remove(&key_code) {
                                (self.settings.callbacks.key_released)(&mut self.render_loop, key_code);
                            }
                        }
                    }
                    PhysicalKey::Unidentified(key) => eprintln!("Unidentified key {:?}", key),
                }
            }
            _ => (),
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _device_id: DeviceId, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta: (x_delta, y_delta) } => {
                if !self.focused {
                    return;
                }
                
                let sensitivity = self.settings.sensitivity;
                let scaled_flipped_delta = (x_delta * sensitivity.0 * 0.01, -y_delta * sensitivity.1 * 0.01);
                (self.settings.callbacks.handle_mouse)(&mut self.render_loop, scaled_flipped_delta);
            }
            DeviceEvent::Key(input) => {
                match input.physical_key {
                    PhysicalKey::Code(key_code) => {
                        if input.state == ElementState::Pressed {
                            (self.settings.callbacks.key_pressed)(&mut self.render_loop, key_code);
                        } else {
                            (self.settings.callbacks.key_released)(&mut self.render_loop, key_code);
                        }
                    }
                    #[allow(unused_variables)]
                    PhysicalKey::Unidentified(key) => {
                        #[cfg(debug_assertions)] eprintln!("key {:?} unidentified", key);
                    }
                }
            }
            _ => {},
        }
    }
}