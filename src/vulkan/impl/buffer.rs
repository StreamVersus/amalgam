use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::utils::BufferUsage;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use vulkan_raw::{vkBindBufferMemory, vkBindBufferMemory2, vkCmdCopyBuffer, vkCmdCopyBufferToImage, vkCmdPipelineBarrier, vkCreateBuffer, vkCreateBufferView, vkDestroyBuffer, vkDestroyBufferView, vkGetBufferMemoryRequirements, vkGetBufferMemoryRequirements2, VkAccessFlags, VkBindBufferMemoryInfo, VkBuffer, VkBufferCopy, VkBufferCreateInfo, VkBufferImageCopy, VkBufferMemoryBarrier, VkBufferMemoryRequirementsInfo2, VkBufferView, VkBufferViewCreateInfo, VkCommandBuffer, VkDependencyFlags, VkFormat, VkFormatFeatureFlags, VkImage, VkImageLayout, VkMemoryDedicatedRequirements, VkMemoryRequirements, VkMemoryRequirements2, VkPipelineStageFlags, VkResult, VkSharingMode, VkVersion, VK_WHOLE_SIZE};

impl Vulkan {
    pub fn create_buffer(&self, size: u64, buffer_usage: BufferUsage) -> Result<VkBuffer, VkResult> {
        let buffer_create_info = VkBufferCreateInfo {
            size,
            usage: buffer_usage.into(),
            sharingMode: VkSharingMode::EXCLUSIVE,
            ..Default::default()
        };
        
        let mut buffer = VkBuffer::none();
        let result = unsafe { vkCreateBuffer(self.get_loaded_device().logical_device, &buffer_create_info, null_mut(), &mut buffer) };
        
        if result == VkResult::SUCCESS {
            Ok(buffer)
        } else {
            Err(result)
        }
    }
    
    pub fn get_buffer_memory_requirements(&self, buffer: &VkBuffer) -> (VkMemoryRequirements, bool) {
        let device = self.get_loaded_device().logical_device;

        let requirement;
        let is_dedicated;
        unsafe {
            if self.is_version_supported(VkVersion::V1_1) {
                let dedicated_info = VkMemoryDedicatedRequirements::default();
                let info = VkBufferMemoryRequirementsInfo2 {
                    pNext: &dedicated_info as *const _ as *const c_void,
                    buffer: *buffer,
                    ..Default::default()
                };

                let mut ph: MaybeUninit<VkMemoryRequirements2> = MaybeUninit::uninit();
                vkGetBufferMemoryRequirements2(device, &info, ph.as_mut_ptr());

                requirement = ph.assume_init().memoryRequirements;
                is_dedicated = dedicated_info.prefersDedicatedAllocation.into() || dedicated_info.requiresDedicatedAllocation.into();
            } else {
                let mut ph: MaybeUninit<VkMemoryRequirements> = MaybeUninit::uninit();
                vkGetBufferMemoryRequirements(device, *buffer, ph.as_mut_ptr());

                requirement = ph.assume_init();
                is_dedicated = false;
            }
            (requirement, is_dedicated)
        }
    }

    pub fn transition_buffers(&self, transition_info: Vec<BufferTransition>, command_buffer: VkCommandBuffer, generating_stages: VkPipelineStageFlags, consuming_stages: VkPipelineStageFlags) {
        let mut buffer_barriers: Vec<VkBufferMemoryBarrier> = Vec::with_capacity(transition_info.len());
        for transition in transition_info {
            buffer_barriers.push(transition.into());
        }
        
        unsafe{ vkCmdPipelineBarrier(command_buffer, generating_stages, consuming_stages, VkDependencyFlags::empty(), 0, null_mut(), buffer_barriers.len() as u32, buffer_barriers.as_ptr(), 0, null_mut()); };
    }
    
    pub fn create_buffer_view(&self, buffer: VkBuffer, format: VkFormat, offset: u64, range: u64) -> VkBufferView {
        let buffer_view_create_info = VkBufferViewCreateInfo {
            buffer,
            format,
            offset,
            range,
            ..Default::default()
        };
        
        let mut buffer_view = VkBufferView::none();
        let result = unsafe { vkCreateBufferView(self.get_loaded_device().logical_device, &buffer_view_create_info, null_mut(), &mut buffer_view ) };
        assert!(result.is_ok());
        
        buffer_view
    }

    fn destroy_buffer_view(&self, buffer_view: VkBufferView) {
        if buffer_view != VkBufferView::none() {
            unsafe { vkDestroyBufferView(self.get_loaded_device().logical_device, buffer_view, null_mut()) };
        }
    }

    fn destroy_buffer(&self, buffer: VkBuffer) {
        if buffer != VkBuffer::none() {
            unsafe { vkDestroyBuffer(self.get_loaded_device().logical_device, buffer, null_mut()) };
        }
    }
    
    /// staging buffer preset
    pub fn create_staging_buffer(&mut self, size: u64) -> VkBuffer {
        self.create_buffer(size, BufferUsage::default().transfer_src(true)).expect("Failed to create buffer")
    }
    
    pub fn create_uniform_texel_buffer(&mut self, format: VkFormat, size: u64, usage: BufferUsage) -> Result<VkBuffer, ()> {
        let format_properties = self.get_format_properties(format);
        
        if !format_properties.bufferFeatures.contains(VkFormatFeatureFlags::UNIFORM_TEXEL_BUFFER_BIT) {
            eprintln!("Provided format is not supported for a uniform texel buffer");
            return Err(());
        }
        
        Ok(self.create_buffer(size, usage.uniform_texel_buffer(true)).expect("Failed to create buffer"))
    }

    pub fn create_storage_texel_buffer(&mut self, format: VkFormat, size: u64, usage: BufferUsage, atomic_operations: bool) -> Result<VkBuffer, ()> {
        let format_properties = self.get_format_properties(format);

        if !format_properties.bufferFeatures.contains(VkFormatFeatureFlags::STORAGE_TEXEL_BUFFER_BIT) {
            eprintln!("Provided format is not supported for a storage texel buffer");
            return Err(());
        }
        if atomic_operations && !format_properties.bufferFeatures.contains(VkFormatFeatureFlags::STORAGE_TEXEL_BUFFER_ATOMIC_BIT) {
            eprintln!("Provided format is not supported for atomic operations on storage texel buffers");
            return Err(());
        }

        Ok(self.create_buffer(size, usage.storage_texel_buffer(true)).expect("Failed to create buffer"))
    }

    pub fn create_uniform_buffer(&mut self, size: u64, usage: BufferUsage) -> VkBuffer {
        self.create_buffer(size, usage.uniform_buffer(true)).expect("Failed to create buffer")
    }

    pub fn create_storage_buffer(&mut self, size: u64, usage: BufferUsage) -> VkBuffer {
        self.create_buffer(size, usage.storage_buffer(true)).expect("Failed to create buffer")
    }
    
    pub fn buffer_to_buffer(&self, regions: Vec<VkBufferCopy>, command_buffer: VkCommandBuffer, src_buffer: VkBuffer, dst_buffer: VkBuffer) {
        if !regions.is_empty() {
            unsafe { vkCmdCopyBuffer(command_buffer, src_buffer, dst_buffer, regions.len() as u32, regions.as_ptr()); };
        }
    }

    pub fn buffer_to_image(&self, regions: Vec<VkBufferImageCopy>, command_buffer: VkCommandBuffer, src_buffer: VkBuffer, dst_image: VkImage, dst_image_layout: VkImageLayout) {
        if !regions.is_empty() {
            unsafe { vkCmdCopyBufferToImage(command_buffer, src_buffer, dst_image, dst_image_layout, regions.len() as u32, regions.as_ptr()); };
        }
    }

    pub fn bind_memory_to_buffer(&self, infos: Vec<VkBindBufferMemoryInfo>) {
        let device = self.get_loaded_device().logical_device;
        if infos.is_empty() {
            return;
        }

        if self.is_version_supported(VkVersion::V1_1) {
            let result = unsafe { vkBindBufferMemory2(device, infos.len() as u32, infos.as_ptr()) };
            assert!(result.is_ok());
        } else {
            for info in infos {
                let result = unsafe { vkBindBufferMemory(device, info.buffer, info.memory, info.memoryOffset) };
                assert!(result.is_ok());
            }
        }
    }
}

pub struct BufferTransition {
    pub buffer: VkBuffer,
    pub current_access: VkAccessFlags,
    pub new_access: VkAccessFlags,
    pub current_queue_family_index: u32,
    pub new_queue_family_index: u32,
    pub size: u64,
    pub offset: u64,
}

impl Default for BufferTransition {
    fn default() -> Self {
        BufferTransition {
            buffer: VkBuffer::none(),
            current_access: VkAccessFlags::empty(),
            new_access: VkAccessFlags::empty(),
            current_queue_family_index: 0,
            new_queue_family_index: 0,
            size: VK_WHOLE_SIZE,
            offset: 0,
        }
    }
}

impl From<BufferTransition> for VkBufferMemoryBarrier {
    fn from(value: BufferTransition) -> Self {
        VkBufferMemoryBarrier {
            srcAccessMask: value.current_access,
            dstAccessMask: value.new_access,
            srcQueueFamilyIndex: value.current_queue_family_index,
            dstQueueFamilyIndex: value.new_queue_family_index,
            buffer: value.buffer,
            offset: value.offset,
            size: value.size,
            ..Default::default()
        }
    }
}

impl Destructible for VkBufferView {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_buffer_view(*self);
    }
}

impl Destructible for VkBuffer {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_buffer(*self);
    }
}