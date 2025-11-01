use crate::safe_ptr;
use crate::vulkan::func::Vulkan;
use crate::vulkan::r#impl::surface::SurfaceFormat;
use crate::vulkan::utils::clamp;
use std::ptr::null_mut;
use vulkan_raw::{vkAcquireNextImageKHR, vkCreateSwapchainKHR, vkDestroySwapchainKHR, vkGetPhysicalDeviceSurfaceFormatsKHR, vkGetSwapchainImagesKHR, vkQueuePresentKHR, VkBool32, VkColorSpaceKHR, VkExtent2D, VkFence, VkFormat, VkImage, VkImageUsageFlagBits, VkImageUsageFlags, VkPhysicalDevice, VkPresentInfoKHR, VkPresentModeKHR, VkQueue, VkResult, VkSemaphore, VkSurfaceFormatKHR, VkSurfaceKHR, VkSurfaceTransformFlagsKHR, VkSwapchainCreateInfoKHR, VkSwapchainKHR};

impl Vulkan {
    fn get_swapchain_image_number(&self, device: VkPhysicalDevice, surface: VkSurfaceKHR) -> u32 {
        let surface_capabilities = self.get_surface_capabilities(device, surface);
        let number_of_image = surface_capabilities.minImageCount + 1;
        if surface_capabilities.maxImageCount > 0 && number_of_image > surface_capabilities.maxImageCount {
            return surface_capabilities.maxImageCount
        }
        number_of_image
    }

    fn get_swapchain_image_size(&self, device: VkPhysicalDevice, surface: VkSurfaceKHR, desired_size: VkExtent2D) -> VkExtent2D {
        let surface_capabilities = self.get_surface_capabilities(device, surface);

        if surface_capabilities.currentExtent.width == 0xFFFFFFFF {
            VkExtent2D {
                width: clamp(desired_size.width, surface_capabilities.minImageExtent.width, surface_capabilities.maxImageExtent.width),
                height: clamp(desired_size.height, surface_capabilities.minImageExtent.height, surface_capabilities.maxImageExtent.height),
            }
        } else {
            surface_capabilities.currentExtent
        }
    }
    
    //TODO: move flags to args
    fn get_desired_usages(&self, device: VkPhysicalDevice, surface: VkSurfaceKHR) -> VkImageUsageFlags {
        let surface_capabilities = self.get_surface_capabilities(device, surface);

        let desired_usage = VkImageUsageFlagBits::COLOR_ATTACHMENT_BIT;
        if surface_capabilities.supportedUsageFlags.contains(desired_usage) {
            desired_usage
        } else {
            panic!("Device doesn't support needed image usage")
        }
    }

    fn get_desired_transformation(&self, device: VkPhysicalDevice, surface: VkSurfaceKHR) -> VkSurfaceTransformFlagsKHR {
        let surface_capabilities = self.get_surface_capabilities(device, surface);

        //TODO: change this to settings contained variable
        let desired_transforms = VkSurfaceTransformFlagsKHR::IDENTITY_BIT_KHR;
        if surface_capabilities.supportedTransforms.contains(desired_transforms) {
            desired_transforms
        } else {
            surface_capabilities.currentTransform
        }
    }

    fn get_image_format(&self, device: VkPhysicalDevice, surface: VkSurfaceKHR, desired_image_format: VkSurfaceFormatKHR) -> VkSurfaceFormatKHR {
        let mut format_count: u32 = 0;

        let mut result = unsafe { vkGetPhysicalDeviceSurfaceFormatsKHR(device, surface, &mut format_count, null_mut()) };
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(format_count, 0);

        let mut surface_formats: Vec<VkSurfaceFormatKHR> = Vec::with_capacity(format_count as usize);
        let spare = surface_formats.spare_capacity_mut();
        unsafe {
            result = vkGetPhysicalDeviceSurfaceFormatsKHR(device, surface, &mut format_count, spare.as_mut_ptr() as *mut VkSurfaceFormatKHR);
        }
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(format_count, 0);

        unsafe {
            surface_formats.set_len(format_count as usize);
        }
        if surface_formats.len() == 1 && surface_formats.get(0).unwrap().format == VkFormat::UNDEFINED {
            return desired_image_format;
        }

        for format in &surface_formats {
            if desired_image_format.format == format.format && desired_image_format.colorSpace == format.colorSpace {
                return desired_image_format;
            }
        };
        dbg!(&surface_formats);
        dbg!(&desired_image_format);
        println!("There is no supported image format, that exactly equals to desired one, falling back to another colorspace");

        for format in &surface_formats {
            if desired_image_format.format == format.format {
                return VkSurfaceFormatKHR {
                    format: desired_image_format.format,
                    colorSpace: format.colorSpace,
                }
            }
        };
        eprintln!("There is no supported image format, falling back to first supported, expect failure");

        surface_formats[0].clone()
    }
    
    pub fn create_swapchain(&self, info: &mut SwapchainInfo) {
        let device = self.get_loaded_device().device;
        let format = self.get_image_format(device, info.surface, info.format.into());
        let image_size = self.get_swapchain_image_size(device, info.surface, VkExtent2D {width: info.width, height: info.height});

        let swapchain_create_info = VkSwapchainCreateInfoKHR {
            surface: info.surface,
            minImageCount: self.get_swapchain_image_number(device, info.surface),
            imageFormat: format.format,
            imageColorSpace: format.colorSpace,
            imageExtent: image_size,
            imageArrayLayers: 1,
            imageUsage: self.get_desired_usages(device, info.surface),
            preTransform: self.get_desired_transformation(device, info.surface),
            presentMode: self.get_presentation_mode({ if info.vsync {
                VkPresentModeKHR::MAILBOX_KHR
            } else {
                VkPresentModeKHR::IMMEDIATE_KHR
            }}, device, info.surface),
            clipped: VkBool32::TRUE,
            oldSwapchain: info.swapchain,
            ..Default::default()
        };

        let mut swapchain = VkSwapchainKHR::none();
        let result = unsafe { vkCreateSwapchainKHR(self.get_loaded_device().logical_device, &swapchain_create_info, null_mut(), &mut swapchain) };
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(swapchain, VkSwapchainKHR::none());
        
        //destroy old one
        self.destroy_swapchain(info.swapchain);
        info.swapchain = swapchain;

        #[cfg(debug_assertions)] println!("Swapchain created with size: {}, {}", image_size.width, image_size.height);
    }
    
    pub fn get_next_image_index(&self, swapchain_info: &SwapchainInfo, semaphore: VkSemaphore, fence: VkFence) -> u32{
        let mut image: u32 = 0;
        let result = unsafe { vkAcquireNextImageKHR(self.get_loaded_device().logical_device, swapchain_info.swapchain, 2000000000, semaphore, fence, &mut image) };
        if result != VkResult::SUCCESS && result != VkResult::SUBOPTIMAL_KHR {
            panic!("Failed to acquire next image: {:?}", result);
        }
        
        image
    }

    pub fn get_images(&self, swapchain_info: &SwapchainInfo) -> Vec<VkImage> {
        let mut swapchain_image_count = 0u32;
        let result = unsafe { vkGetSwapchainImagesKHR(self.get_loaded_device().logical_device, swapchain_info.swapchain, &mut swapchain_image_count, null_mut()) };
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(swapchain_image_count, 0);

        let mut images = Vec::with_capacity(swapchain_image_count as usize);
        let result = unsafe { vkGetSwapchainImagesKHR(self.get_loaded_device().logical_device, swapchain_info.swapchain, &mut swapchain_image_count, images.as_mut_ptr()) };
        assert_eq!(result, VkResult::SUCCESS);
        assert_ne!(swapchain_image_count, 0);

        unsafe { images.set_len(swapchain_image_count as usize) };
        images
    }
    
    pub fn present_images(&self, queue: VkQueue, present_info: Vec<PresentInfo>) {
        let mut swapchains: Vec<VkSwapchainKHR> = Vec::with_capacity(present_info.len());
        let mut image_indices: Vec<u32> = Vec::with_capacity(present_info.len());
        let mut semaphores: Vec<VkSemaphore> = Vec::with_capacity(present_info.len());
        for info in present_info {
            swapchains.push(info.swapchain);
            image_indices.push(info.image_index);
            semaphores.push(info.semaphore);
        }
        
        let image_presentation_info = VkPresentInfoKHR {
            waitSemaphoreCount: semaphores.len() as u32,
            pWaitSemaphores: safe_ptr!(semaphores),
            swapchainCount: swapchains.len() as u32,
            pSwapchains: safe_ptr!(swapchains),
            pImageIndices: safe_ptr!(image_indices),
            ..Default::default()
        };
        
        let result = unsafe { vkQueuePresentKHR(queue, &image_presentation_info) };
        assert_eq!(result, VkResult::SUCCESS);
    }
    
    pub fn destroy_swapchain(&self, swapchain: VkSwapchainKHR) {
        if swapchain != VkSwapchainKHR::none() {
            unsafe { vkDestroySwapchainKHR(self.get_loaded_device().logical_device, swapchain, null_mut()) };
        }
    }
}

pub struct PresentInfo {
    pub swapchain: VkSwapchainKHR,
    pub image_index: u32,
    pub semaphore: VkSemaphore,
}

pub struct SwapchainInfo {
    pub width: u32,
    pub height: u32,
    pub surface: VkSurfaceKHR,
    pub format: SurfaceFormat,
    pub swapchain: VkSwapchainKHR,
    pub vsync: bool,
}

impl Default for SwapchainInfo {
    fn default() -> SwapchainInfo {
        SwapchainInfo {
            width: 640,
            height: 480,
            surface: Default::default(),
            format: SurfaceFormat {
                format: VkFormat::B8G8R8A8_UNORM,
                colorSpace: VkColorSpaceKHR::SRGB_NONLINEAR_KHR,
            },
            swapchain: VkSwapchainKHR::none(),
            vsync: false,
        }
    }
}

impl SwapchainInfo {
    pub fn set_width(&mut self, width: u32) -> &mut Self{
        self.width = width;
        self
    }
    
    pub fn set_height(&mut self, height: u32) -> &mut Self{
        self.height = height;
        self
    }
    
    pub fn set_format(&mut self, format: SurfaceFormat) -> &mut Self{
        self.format = format;
        self
    }
    
    pub fn set_swapchain(&mut self, swapchain: VkSwapchainKHR) -> &mut Self{
        self.swapchain = swapchain;
        self
    }
    
    pub fn set_vsync(&mut self, vsync: bool) -> &mut Self{
        self.vsync = vsync;
        self
    }
    
    pub fn set_surface(&mut self, surface: VkSurfaceKHR) -> &mut Self{
        self.surface = surface;
        self
    }
}
