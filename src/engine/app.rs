use crate::application::WINDOW_TITLE;
use crate::both::RenderLoop;
use crate::engine::utils::gui::{egui_to_winit_cursor, EGuiMediator};
use crate::engine::{Delta, FrameInfo, Settings};
use crate::prelude::*;
use crate::vulkan::func::Vulkan;
use egui::{PlatformOutput, Vec2};
use std::collections::HashSet;
use std::ptr;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

#[derive(Default)]
pub struct App {
    pub window: Option<Rc<Box<dyn Window>>>,
    pub delta: Delta,
    pub settings: Settings,
    pub vulkan: Vulkan,
    pub swapchain_info: SwapchainInfo,
    pub render_loop: RenderLoop,
    pub pressed_keys: HashSet<KeyCode>,
    pub handler: Option<WinitHandler>,
    pub egui: EGuiMediator,

    pub focused: bool,
}

pub struct WinitHandler {
    pub window: Rc<Box<dyn Window>>,
}

impl WinitHandler {
    pub fn handle_output(&mut self, platform_output: PlatformOutput) {
        self.window.set_cursor(egui_to_winit_cursor(platform_output.cursor_icon));
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let window_attrs = WindowAttributes::default()
            .with_title(WINDOW_TITLE)
            .with_surface_size(PhysicalSize::new(self.settings.width, self.settings.height))
            .with_visible(true);
        let window = event_loop.create_window(window_attrs).unwrap();
        let scale = window.scale_factor() as f32;

        let window_rc = Rc::new(window);
        self.window = Some(window_rc.clone());
        self.egui = EGuiMediator::init(Vec2::new(self.settings.width as f32, self.settings.height as f32), scale);
        println!("egui setup finished");


        self.handler = Some(WinitHandler {
             window: window_rc.clone(),
        });
        let surface = self.vulkan.connect_vulkan(self.window.as_ref().unwrap());
        self.swapchain_info
            .set_width(self.settings.width)
            .set_height(self.settings.height)
            .set_format(self.settings.render_format)
            .set_surface(surface);
        self.vulkan.create_swapchain(&mut self.swapchain_info);

        (self.settings.callbacks.render_init)(&mut self.render_loop, &mut self.vulkan, &mut self.swapchain_info, &mut self.settings);
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        self.egui.handle_window_event(event.clone());
        match event {
            WindowEvent::CloseRequested => {
                self.vulkan.destroy_swapchain(self.swapchain_info.swapchain);
                self.swapchain_info.swapchain = VkSwapchainKHR::none();

                self.vulkan.destroy_surface(self.swapchain_info.surface);
                self.swapchain_info.surface = VkSurfaceKHR::none();

                unsafe {
                    let mut stats_string: *mut i8 = ptr::null_mut();
                    vmaBuildStatsString(self.vulkan.pool().allocator(), &mut stats_string, VkBool32::TRUE);

                    let cstr = std::ffi::CStr::from_ptr(stats_string);
                    println!("{}", cstr.to_string_lossy());

                    vmaFreeStatsString(self.vulkan.pool().allocator(), stats_string); // must free it

                    let mut stats = VmaTotalStatistics::default();
                    vmaCalculateStatistics(self.vulkan.pool().allocator(), &mut stats);

                    println!("=== VMA Stats ===");
                    println!("total allocations:  {}", stats.total.statistics.allocationCount);
                    println!("total alloc bytes:  {}", stats.total.statistics.allocationBytes);
                    println!("total blocks:       {}", stats.total.statistics.blockCount);
                    println!("total block bytes:  {}", stats.total.statistics.blockBytes);
                    println!("unused range count: {}", stats.total.unusedRangeCount);
                    println!("alloc size min:     {}", stats.total.allocationSizeMin);
                    println!("alloc size max:     {}", stats.total.allocationSizeMax);
                    println!("unused range min:   {}", stats.total.unusedRangeSizeMin);
                    println!("unused range max:   {}", stats.total.unusedRangeSizeMax);
                }

                self.vulkan.device_wait();
                self.render_loop = RenderLoop::default();
                
                self.vulkan.pool().finish();
                self.vulkan.finish();

                event_loop.exit();
            },
            WindowEvent::SurfaceResized(size) => {
                self.swapchain_info.set_width(size.width).set_height(size.height);
            },
            WindowEvent::Focused(focused) => {
                let window = self.window.as_ref().unwrap();
                if focused {
                    window.set_cursor_visible(false);
                    window.set_cursor_grab(CursorGrabMode::Locked)
                        .unwrap_or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined).unwrap_or_else(|_| {
                            eprintln!("FAILED TO CATCH CURSOR");
                        }));
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
                        } else if self.pressed_keys.remove(&key_code)  {
                            (self.settings.callbacks.key_released)(&mut self.render_loop, key_code);
                        }
                    }
                    PhysicalKey::Unidentified(key) => eprintln!("Unidentified key {:?}", key),
                }
            },
            WindowEvent::RedrawRequested => {
                let frame_info = FrameInfo {
                    delta_time: self.delta.tick(),
                    raw_input: self.egui.take_egui_input(),
                };

                (self.settings.callbacks.render)(&mut self.render_loop, &mut self.vulkan, &mut self.swapchain_info, &mut self.egui.ctx, self.handler.as_mut().unwrap(), frame_info);
                self.delta.sleep_till_next_frame();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }

    fn device_event(&mut self, _event_loop: &dyn ActiveEventLoop, _device_id: Option<DeviceId>, event: DeviceEvent) {
        self.egui.handle_device_event(event.clone());
        match event {
            DeviceEvent::PointerMotion { delta: (x_delta, y_delta) } => {
                if !self.focused {
                    return;
                }

                let sensitivity = self.settings.sensitivity;
                let scaled_flipped_delta = (x_delta * sensitivity.0, -y_delta * sensitivity.1);
                (self.settings.callbacks.handle_mouse)(&mut self.render_loop, scaled_flipped_delta);
            },
            _ => {},
        }
    }
}