use crate::safe_ptr;
use crate::vulkan::func::{bool_to_vkbool, Destructible, Vulkan};
use std::ptr::null_mut;
use vulkan_raw::{vkAllocateCommandBuffers, vkBeginCommandBuffer, vkCreateCommandPool, vkDestroyCommandPool, vkEndCommandBuffer, vkFreeCommandBuffers, vkQueueSubmit, vkResetCommandBuffer, vkResetCommandPool, VkCommandBuffer, VkCommandBufferAllocateInfo, VkCommandBufferBeginInfo, VkCommandBufferInheritanceInfo, VkCommandBufferLevel, VkCommandBufferResetFlagBits, VkCommandBufferResetFlags, VkCommandBufferUsageFlags, VkCommandPool, VkCommandPoolCreateFlags, VkCommandPoolCreateInfo, VkCommandPoolResetFlags, VkFence, VkFramebuffer, VkPipelineStageFlags, VkQueryControlFlags, VkQueryPipelineStatisticFlags, VkQueue, VkRenderPass, VkSemaphore, VkSubmitInfo};

impl Vulkan {
    pub fn create_command_pool(&self, family_index: u32, flags: VkCommandPoolCreateFlags) -> VkCommandPool {
        let command_pool_create_info = VkCommandPoolCreateInfo {
            flags,
            queueFamilyIndex: family_index,
            ..Default::default()
        };
        let mut command_pool = VkCommandPool::none();
        let result = unsafe { vkCreateCommandPool(self.get_loaded_device().logical_device, &command_pool_create_info, null_mut(), &mut command_pool) };
        assert!(result.is_ok());
        assert_ne!(command_pool, VkCommandPool::none());

        command_pool
    }

    pub fn alloc_command_buffers(&self, command_pool: VkCommandPool, level: VkCommandBufferLevel, amount: u32) -> Vec<VkCommandBuffer> {
        let command_buffer_alloc_info = VkCommandBufferAllocateInfo {
            commandPool: command_pool,
            level,
            commandBufferCount: amount,
            ..Default::default()
        };
        let mut buffers = Vec::with_capacity(amount as usize);
        let spare = buffers.spare_capacity_mut();

        let result = unsafe {
            vkAllocateCommandBuffers(self.get_loaded_device().logical_device, &command_buffer_alloc_info, spare.as_mut_ptr() as *mut VkCommandBuffer)
        };
        assert!(result.is_ok());
        
        unsafe {
            buffers.set_len(amount as usize);
        }
        buffers
    }

    pub fn start_recording(&self, command_buffer: VkCommandBuffer, usage: VkCommandBufferUsageFlags, info: RecordingInfo) {
        let secondary_command_buffer_inheritance_info: VkCommandBufferInheritanceInfo = info.into();

        let command_buffer_begin_info = VkCommandBufferBeginInfo {
            flags: usage,
            pInheritanceInfo: &secondary_command_buffer_inheritance_info,
            ..Default::default()
        };

        let result = unsafe { vkBeginCommandBuffer(command_buffer, &command_buffer_begin_info) };
        assert!(result.is_ok());
    }

    pub fn end_recording(&self, command_buffer: VkCommandBuffer) {
        let result = unsafe { vkEndCommandBuffer(command_buffer) };
        assert!(result.is_ok());
    }

    pub fn reset_buffer(&self, command_buffer: VkCommandBuffer, release_resources: bool) {
        let result = unsafe { vkResetCommandBuffer(command_buffer, {
            if release_resources {
                VkCommandBufferResetFlagBits::RELEASE_RESOURCES_BIT
            } else {
                VkCommandBufferResetFlags::empty()
            }
        }) };
        assert!(result.is_ok());
    }

    pub fn reset_pool(&self, command_pool: VkCommandPool, release_resources: bool) {
        let result = unsafe { vkResetCommandPool(self.get_loaded_device().logical_device, command_pool, {
            if release_resources {
                VkCommandPoolResetFlags::RELEASE_RESOURCES_BIT
            } else {
                VkCommandPoolResetFlags::empty()
            }
        }) };
        assert!(result.is_ok());
    }
    
    pub fn destroy_pool(&self, command_pool: VkCommandPool) {
        if command_pool != VkCommandPool::none() {
            unsafe { vkDestroyCommandPool(self.get_loaded_device().logical_device, command_pool, null_mut()) };
        }
    }
    
    pub fn free_buffers(&self, command_pool: VkCommandPool, command_buffers: &[VkCommandBuffer]) {
        if command_buffers.len() > 0 {
            unsafe { vkFreeCommandBuffers(self.get_loaded_device().logical_device, command_pool, command_buffers.len() as u32, command_buffers.as_ptr()) };
        }
    }
    
    pub fn submit_buffer(&self, queue: VkQueue, fence: VkFence, command_buffers: &[VkCommandBuffer], wait_semaphores: &[WaitSemaphoreInfo], signal_semaphores: &[VkSemaphore]) {
        let mut semaphores: Vec<VkSemaphore> = Vec::with_capacity(wait_semaphores.len());
        let mut stages: Vec<VkPipelineStageFlags> = Vec::with_capacity(wait_semaphores.len());

        for wait_info in wait_semaphores {
            semaphores.push(wait_info.semaphore);
            stages.push(wait_info.waiting_stage);
        }
        
        let submit_info = VkSubmitInfo {
            waitSemaphoreCount: wait_semaphores.len() as u32,
            pWaitSemaphores: safe_ptr!(semaphores),
            pWaitDstStageMask: safe_ptr!(stages),
            commandBufferCount: command_buffers.len() as u32,
            pCommandBuffers: safe_ptr!(command_buffers),
            signalSemaphoreCount: signal_semaphores.len() as u32,
            pSignalSemaphores: safe_ptr!(signal_semaphores),
            ..Default::default()
        };
        
        let result = unsafe { vkQueueSubmit(queue, 1, &submit_info, fence) };
        assert!(result.is_ok());
    }
}

#[allow(non_snake_case)]
pub struct RecordingInfo {
    pub renderPass: VkRenderPass,
    pub subpass: u32,
    pub framebuffer: VkFramebuffer,
    pub occlusionQueryEnable: bool,
    pub queryFlags: VkQueryControlFlags,
    pub pipelineStatistics: VkQueryPipelineStatisticFlags,
}

pub struct WaitSemaphoreInfo {
    pub semaphore: VkSemaphore,
    pub waiting_stage: VkPipelineStageFlags,
}

impl Into<VkCommandBufferInheritanceInfo> for RecordingInfo {
    fn into(self) -> VkCommandBufferInheritanceInfo {
        VkCommandBufferInheritanceInfo {
            renderPass: self.renderPass,
            subpass: self.subpass,
            framebuffer: self.framebuffer,
            occlusionQueryEnable: bool_to_vkbool(self.occlusionQueryEnable),
            queryFlags: self.queryFlags,
            pipelineStatistics: self.pipelineStatistics,
            ..Default::default()
        }
    }
}

impl Destructible for VkCommandPool {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_pool(*self)
    }
}