use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::utils::BufferUsage;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use vulkan_raw::{vkBindBufferMemory, vkBindBufferMemory2, vkCmdCopyBuffer, vkCmdCopyBufferToImage, vkCmdPipelineBarrier, vkCmdPipelineBarrier2, vkCreateBuffer, vkCreateBufferView, vkDestroyBuffer, vkDestroyBufferView, vkGetBufferMemoryRequirements, vkGetBufferMemoryRequirements2, VkAccessFlags, VkAccessFlags2, VkBindBufferMemoryInfo, VkBuffer, VkBufferCopy, VkBufferCreateInfo, VkBufferImageCopy, VkBufferMemoryBarrier, VkBufferMemoryBarrier2, VkBufferMemoryRequirementsInfo2, VkBufferView, VkBufferViewCreateInfo, VkCommandBuffer, VkDependencyFlags, VkDependencyInfo, VkDeviceSize, VkFormat, VkFormatFeatureFlags, VkImage, VkImageLayout, VkMemoryDedicatedRequirements, VkMemoryRequirements, VkMemoryRequirements2, VkPipelineStageFlags, VkPipelineStageFlags2, VkResult, VkSharingMode, VkStructureType, VkVersion, VK_WHOLE_SIZE};

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

    pub fn transition_buffers(
        &self,
        transition_info: Vec<BufferTransition>,
        command_buffer: VkCommandBuffer
    ) {
        if transition_info.is_empty() { return; }

        let buffer_barriers: Vec<VkBufferMemoryBarrier2> = transition_info
            .into_iter()
            .map(|t| t.into())
            .collect();

        let dependency_info = VkDependencyInfo {
            dependencyFlags: VkDependencyFlags::empty(),
            memoryBarrierCount: 0,
            bufferMemoryBarrierCount: buffer_barriers.len() as u32,
            pBufferMemoryBarriers: buffer_barriers.as_ptr(),
            ..Default::default()
        };

        unsafe { vkCmdPipelineBarrier2(command_buffer, &dependency_info); }
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
    
    pub fn buffer_to_buffer(&self, regions: &[VkBufferCopy], command_buffer: VkCommandBuffer, src_buffer: VkBuffer, dst_buffer: VkBuffer) {
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
    pub offset: VkDeviceSize,
    pub size: VkDeviceSize,
    pub src_stage: VkPipelineStageFlags2,
    pub dst_stage: VkPipelineStageFlags2,
    pub src_access: VkAccessFlags2,
    pub dst_access: VkAccessFlags2,
    pub src_queue_family: u32,
    pub dst_queue_family: u32,
}

impl Default for BufferTransition {
    fn default() -> Self {
        BufferTransition {
            buffer: VkBuffer::none(),
            size: VK_WHOLE_SIZE,
            src_stage: Default::default(),
            dst_stage: Default::default(),
            src_access: Default::default(),
            dst_access: Default::default(),
            src_queue_family: 0,
            offset: 0,
            dst_queue_family: 0,
        }
    }
}

impl Into<VkBufferMemoryBarrier2> for BufferTransition {
    fn into(self) -> VkBufferMemoryBarrier2 {
        VkBufferMemoryBarrier2 {
            srcStageMask: self.src_stage,
            srcAccessMask: self.src_access,
            dstStageMask: self.dst_stage,
            dstAccessMask: self.dst_access,
            srcQueueFamilyIndex: self.src_queue_family,
            dstQueueFamilyIndex: self.dst_queue_family,
            buffer: self.buffer,
            offset: self.offset,
            size: self.size,
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