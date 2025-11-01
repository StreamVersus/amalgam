use crate::vulkan::func::Vulkan;
use std::ptr::null_mut;
use vulkan_raw::{vkCreateAndroidSurfaceKHR, vkCreateWaylandSurfaceKHR, vkCreateWin32SurfaceKHR, vkCreateXcbSurfaceKHR, vkCreateXlibSurfaceKHR, vkDestroySurfaceKHR, vkGetPhysicalDeviceSurfaceCapabilitiesKHR, vkGetPhysicalDeviceSurfacePresentModesKHR, VkAndroidSurfaceCreateInfoKHR, VkColorSpaceKHR, VkFormat, VkPhysicalDevice, VkPresentModeKHR, VkResult, VkSurfaceCapabilitiesKHR, VkSurfaceFormatKHR, VkSurfaceKHR, VkWaylandSurfaceCreateInfoKHR, VkWin32SurfaceCreateInfoKHR, VkXcbSurfaceCreateInfoKHR, VkXlibSurfaceCreateInfoKHR, HINSTANCE, HWND};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use winit::window::Window;

impl Vulkan {
    pub fn get_presentation_mode(&self, desired_mode: VkPresentModeKHR, device: VkPhysicalDevice, surface: VkSurfaceKHR) -> VkPresentModeKHR {
        let mut present_mode_count = 0;
        let mut result = unsafe {
            vkGetPhysicalDeviceSurfacePresentModesKHR(device, surface, &mut present_mode_count, null_mut())
        };
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(present_mode_count, 0);
        
        let mut present_modes: Vec<VkPresentModeKHR> = Vec::with_capacity(present_mode_count as usize);
        let spare = present_modes.spare_capacity_mut();
        result = unsafe {
            vkGetPhysicalDeviceSurfacePresentModesKHR(device, surface, &mut present_mode_count, spare.as_mut_ptr() as *mut VkPresentModeKHR)
        };
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(present_mode_count, 0);

        unsafe {
            present_modes.set_len(present_mode_count as usize);
        }
        for mode in &present_modes {
            if *mode == desired_mode {
                return desired_mode;
            }
        }
        println!("Desired present mode is unsupported, falling back to FIFO");
        
        for mode in &present_modes {
            if *mode == VkPresentModeKHR::FIFO_KHR {
                return VkPresentModeKHR::FIFO_KHR;
            }
        }
        panic!("FIFO is unsupported, is vulkan hardware supports graphic?");
    }

    pub fn get_surface_capabilities(&self, device: VkPhysicalDevice, surface: VkSurfaceKHR) -> VkSurfaceCapabilitiesKHR {
        let mut surface_capabilities = VkSurfaceCapabilitiesKHR::default();

        let result = unsafe { vkGetPhysicalDeviceSurfaceCapabilitiesKHR(device, surface, &mut surface_capabilities) };
        assert_eq!(result, VkResult::SUCCESS);

        surface_capabilities
    }
    
    pub fn connect_vulkan(&self, window: &Window) -> VkSurfaceKHR {
        let instance = self.get_instance();
        
        let display_handle = window.display_handle().unwrap().as_raw();
        let window_handle = window.window_handle().unwrap().as_raw();
        unsafe {
            //maybe rewrite someday, not like its incredibly bad, just not so clean
            match (display_handle, window_handle) {
                (RawDisplayHandle::Xcb(display), RawWindowHandle::Xcb(window)) => {
                    let surface_create_info = VkXcbSurfaceCreateInfoKHR {
                        connection: display.connection.unwrap().as_ptr(),
                        window: window.window.get(),
                        ..Default::default()
                    };

                    let mut surface = VkSurfaceKHR::none();
                    let result = vkCreateXcbSurfaceKHR(instance, &surface_create_info, null_mut(), &mut surface);
                    assert_eq!(result, VkResult::SUCCESS);

                    surface
                }
                (RawDisplayHandle::Xlib(display), RawWindowHandle::Xlib(window)) => {
                    let surface_create_info = VkXlibSurfaceCreateInfoKHR {
                        dpy: display.display.unwrap().as_ptr(),
                        window: window.window as usize,
                        ..Default::default()
                    };

                    let mut surface = VkSurfaceKHR::none();
                    let result = vkCreateXlibSurfaceKHR(instance, &surface_create_info, null_mut(), &mut surface);
                    assert_eq!(result, VkResult::SUCCESS);

                    surface
                }
                (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(window)) => {
                    let surface_create_info = VkWaylandSurfaceCreateInfoKHR {
                        display: display.display.as_ptr(),
                        surface: window.surface.as_ptr(),
                        ..Default::default()
                    };

                    let mut surface = VkSurfaceKHR::none();
                    let result = vkCreateWaylandSurfaceKHR(instance, &surface_create_info, null_mut(), &mut surface);
                    assert_eq!(result, VkResult::SUCCESS);

                    surface
                }
                (RawDisplayHandle::Windows(_), RawWindowHandle::Win32(window)) => {
                    let surface_create_info = VkWin32SurfaceCreateInfoKHR {
                        hinstance: window.hinstance.unwrap().get() as HINSTANCE,
                        hwnd: window.hwnd.get() as HWND,
                        ..Default::default()
                    };

                    let mut surface = VkSurfaceKHR::none();
                    let result = vkCreateWin32SurfaceKHR(instance, &surface_create_info, null_mut(), &mut surface);
                    assert_eq!(result, VkResult::SUCCESS);

                    surface
                }
                (RawDisplayHandle::Android(_), RawWindowHandle::AndroidNdk(window)) => {
                    let surface_create_info = VkAndroidSurfaceCreateInfoKHR {
                        window: window.a_native_window.as_ptr(),
                        ..Default::default()
                    };

                    let mut surface = VkSurfaceKHR::none();
                    let result = vkCreateAndroidSurfaceKHR(instance, &surface_create_info, null_mut(), &mut surface);
                    assert_eq!(result, VkResult::SUCCESS);

                    surface
                }
                (_, _) => {
                    panic!("Unsupported platform!");
                }
            }
        }
    }
    
    pub fn destroy_surface(&self, surface: VkSurfaceKHR) {
        if surface != VkSurfaceKHR::none() {
            unsafe { vkDestroySurfaceKHR(self.get_instance(), surface, null_mut()) };
        }
    }
}

#[derive(Copy, Clone)]
#[allow(non_snake_case)]
pub struct SurfaceFormat {
    pub format: VkFormat,
    pub colorSpace: VkColorSpaceKHR,
}
impl Default for SurfaceFormat {
    fn default() -> Self {
        SurfaceFormat {
            format: VkFormat::B8G8R8A8_UNORM,
            colorSpace: VkColorSpaceKHR::SRGB_NONLINEAR_KHR,
        }
    }
}

impl Into<VkSurfaceFormatKHR> for SurfaceFormat {
    fn into(self) -> VkSurfaceFormatKHR {
        VkSurfaceFormatKHR {
            format: self.format,
            colorSpace: self.colorSpace,
        }
    }
}
