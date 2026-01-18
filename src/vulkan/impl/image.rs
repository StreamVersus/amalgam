use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::utils::ImageUsage;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use vulkan_raw::{vkBindImageMemory, vkBindImageMemory2, vkCmdClearColorImage, vkCmdClearDepthStencilImage, vkCmdCopyImageToBuffer, vkCmdPipelineBarrier, vkCreateImage, vkCreateImageView, vkDestroyImage, vkDestroyImageView, vkGetImageMemoryRequirements, vkGetImageMemoryRequirements2, VkAccessFlags, VkBindImageMemoryInfo, VkBuffer, VkBufferImageCopy, VkClearColorValue, VkClearDepthStencilValue, VkCommandBuffer, VkDependencyFlags, VkExtent2D, VkExtent3D, VkFormat, VkFormatFeatureFlagBits, VkImage, VkImageAspectFlags, VkImageCreateFlagBits, VkImageCreateInfo, VkImageLayout, VkImageMemoryBarrier, VkImageMemoryRequirementsInfo2, VkImageSubresourceRange, VkImageTiling, VkImageType, VkImageView, VkImageViewCreateInfo, VkImageViewType, VkMemoryDedicatedRequirements, VkMemoryRequirements, VkMemoryRequirements2, VkPipelineStageFlags, VkResult, VkSampleCountFlagBits, VkSampleCountFlags, VkSharingMode, VkVersion, VK_REMAINING_ARRAY_LAYERS, VK_REMAINING_MIP_LEVELS};

impl Vulkan {
    pub fn create_image(&self, format: VkFormat, image_type: VkImageType, is_cubemap: bool, mipmaps: u32, layers: u32, size: VkExtent3D, samples: VkSampleCountFlags, usage: ImageUsage) -> VkImage {
        let image_create_info = VkImageCreateInfo {
            flags: { 
                if is_cubemap {
                    VkImageCreateFlagBits::CUBE_COMPATIBLE_BIT
                } else {
                    VkImageCreateFlagBits::empty()
                }
            },
            imageType: image_type,
            format,
            extent: size,
            mipLevels: mipmaps,
            arrayLayers: layers,
            samples,
            tiling: VkImageTiling::OPTIMAL,
            usage: usage.into(),
            sharingMode: VkSharingMode::EXCLUSIVE,
            ..Default::default()
        };
        
        let mut image = VkImage::none();
        let result = unsafe { vkCreateImage(self.get_loaded_device().logical_device, &image_create_info, null_mut(), &mut image) };
        assert_eq!(VkResult::SUCCESS, result);
        
        image
    }
    
    pub fn transition_images(&self, transition_infos: Vec<ImageTransition>, command_buffer: VkCommandBuffer, generating_stages: VkPipelineStageFlags, consuming_stages: VkPipelineStageFlags) {
        let mut image_barriers: Vec<VkImageMemoryBarrier> = Vec::with_capacity(transition_infos.len());
        for transition in transition_infos {
            image_barriers.push(transition.into());
        }

        unsafe { vkCmdPipelineBarrier(command_buffer, generating_stages, consuming_stages, VkDependencyFlags::empty(), 0, null_mut(), 0, null_mut(), image_barriers.len() as u32, image_barriers.as_ptr()); };
    }

    pub fn create_image_view(&self, image: &VkImage, view_type: VkImageViewType, format: VkFormat, aspect: VkImageAspectFlags) -> VkImageView {
        let image_view_create_info = VkImageViewCreateInfo {
            image: *image,
            viewType: view_type,
            format,
            subresourceRange: VkImageSubresourceRange {
                aspectMask: aspect,
                levelCount: VK_REMAINING_MIP_LEVELS,
                layerCount: VK_REMAINING_ARRAY_LAYERS,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut image_view = VkImageView::none();
        let result = unsafe { vkCreateImageView(self.get_loaded_device().logical_device, &image_view_create_info, null_mut(), &mut image_view) };
        assert!(result.is_ok());

        image_view
    }

    fn destroy_image_view(&self, image_view: VkImageView) {
        if image_view != VkImageView::none() {
            unsafe { vkDestroyImageView(self.get_loaded_device().logical_device, image_view, null_mut()) };
        }
    }

    fn destroy_image(&self, image: VkImage) {
        if image != VkImage::none() {
            unsafe { vkDestroyImage(self.get_loaded_device().logical_device, image, null_mut()) };
        }
    }

    pub fn get_image_memory_requirements(&self, image: &VkImage) -> (VkMemoryRequirements, bool) {
        let device = self.get_loaded_device().logical_device;

        let requirement;
        let is_dedicated;
        unsafe {
            if self.is_version_supported(VkVersion::V1_1) {
                let dedicated_info = VkMemoryDedicatedRequirements::default();
                let info = VkImageMemoryRequirementsInfo2 {
                    pNext: &dedicated_info as *const _ as *const c_void,
                    image: *image,
                    ..Default::default()
                };

                let mut ph: MaybeUninit<VkMemoryRequirements2> = MaybeUninit::uninit();
                vkGetImageMemoryRequirements2(device, &info, ph.as_mut_ptr());

                is_dedicated = dedicated_info.prefersDedicatedAllocation.into() || dedicated_info.requiresDedicatedAllocation.into();
                requirement = ph.assume_init().memoryRequirements;
            } else {
                let mut ph: MaybeUninit<VkMemoryRequirements> = MaybeUninit::uninit();
                vkGetImageMemoryRequirements(device, *image, ph.as_mut_ptr());

                requirement = ph.assume_init();
                is_dedicated = false;
            }
            (requirement, is_dedicated)
        }
    }

    pub fn create_2d_image_and_view(&mut self, format: VkFormat, mipmaps: u32, layers: u32, size: VkExtent2D, samples: VkSampleCountFlags, usage: ImageUsage) -> VkImage {
        let size = VkExtent3D {
            width: size.width,
            height: size.height,
            depth: 1,
        };
        self.create_image(format, VkImageType::IT_2D, false, mipmaps, layers, size, samples, usage)
    }

    pub fn create_sampled_image(&mut self, format: VkFormat, image_type: VkImageType, mipmaps: u32, layers: u32, size: VkExtent2D, usage: ImageUsage, linear_filtering: bool) -> Result<VkImage, ()> {
        let format_properties = self.get_format_properties(format);
        if !format_properties.optimalTilingFeatures.contains(VkFormatFeatureFlagBits::SAMPLED_IMAGE_BIT) {
            eprintln!("Provided format doesn't supported for sampled image");
            return Err(());
        }
        if linear_filtering && !format_properties.optimalTilingFeatures.contains(VkFormatFeatureFlagBits::SAMPLED_IMAGE_FILTER_LINEAR_BIT) {
            eprintln!("Provided format doesn't support linear filtering");
            return Err(());
        }
        let size = VkExtent3D {
            width: size.width,
            height: size.height,
            depth: 1,
        };
        Ok(self.create_image(format, image_type, false, mipmaps, layers, size, VkSampleCountFlagBits::SC_1_BIT, usage.sampled(true)))
    }

    pub fn image_to_buffer(&self, regions: Vec<VkBufferImageCopy>, command_buffer: VkCommandBuffer, src_image: VkImage, dst_buffer: VkBuffer, src_image_layout: VkImageLayout) {
        if regions.len() > 0 {
            unsafe { vkCmdCopyImageToBuffer(command_buffer, src_image, src_image_layout, dst_buffer, regions.len() as u32, regions.as_ptr()) };
        }
    }

    pub fn bind_memory_to_image(&self, infos: Vec<VkBindImageMemoryInfo>) {
        let device = self.get_loaded_device().logical_device;
        if infos.len() == 0 {
            return;
        }
        
        if self.is_version_supported(VkVersion::V1_1) {
            let result = unsafe { vkBindImageMemory2(device, infos.len() as u32, infos.as_ptr()) };
            assert!(result.is_ok());
        } else {
            for info in infos {
                let result = unsafe { vkBindImageMemory(device, info.image, info.memory, info.memoryOffset) };
                assert!(result.is_ok());
            }
        }
    }

    pub fn clear_color_image(&self, image: VkImage, image_layout: VkImageLayout, clear_color: VkClearColorValue, ranges: Vec<VkImageSubresourceRange>, command_buffer: VkCommandBuffer) {
        unsafe { vkCmdClearColorImage(command_buffer, image, image_layout, &clear_color, ranges.len() as u32, ranges.as_ptr()) };
    }

    pub fn clear_depth_stencil(&self, image: VkImage, image_layout: VkImageLayout, clear_color: VkClearDepthStencilValue, ranges: Vec<VkImageSubresourceRange>, command_buffer: VkCommandBuffer) {
        unsafe { vkCmdClearDepthStencilImage(command_buffer, image, image_layout, &clear_color, ranges.len() as u32, ranges.as_ptr()) };
    }
}

pub struct ImageTransition {
    pub image: VkImage,
    pub current_access: VkAccessFlags,
    pub new_access: VkAccessFlags,
    pub current_layout: VkImageLayout,
    pub new_layout: VkImageLayout,
    pub current_queue_family: u32,
    pub new_queue_family: u32,
    pub aspect: VkImageAspectFlags,
}

impl Default for ImageTransition {
    fn default() -> ImageTransition {
        ImageTransition {
            image: VkImage::none(),
            current_access: VkAccessFlags::empty(),
            new_access: VkAccessFlags::empty(),
            current_layout: VkImageLayout::UNDEFINED,
            new_layout: VkImageLayout::UNDEFINED,
            current_queue_family: 0,
            new_queue_family: 0,
            aspect: VkImageAspectFlags::empty(),
        }
    }
}

impl Into<VkImageMemoryBarrier> for ImageTransition {
    fn into(self) -> VkImageMemoryBarrier {
        VkImageMemoryBarrier {
            srcAccessMask: self.current_access,
            dstAccessMask: self.new_access,
            oldLayout: self.current_layout,
            newLayout: self.new_layout,
            srcQueueFamilyIndex: self.current_queue_family,
            dstQueueFamilyIndex: self.new_queue_family,
            image: self.image,
            subresourceRange: VkImageSubresourceRange {
                aspectMask: self.aspect,
                levelCount: 1,
                layerCount: 1,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl Destructible for VkImageView {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_image_view(*self);
    }
}

impl Destructible for VkImage {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_image(*self);
    }
}