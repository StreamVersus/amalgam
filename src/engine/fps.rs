use std::ffi::c_void;
use vulkan_raw::{vkCmdResetQueryPool, vkCmdWriteTimestamp, vkCreateQueryPool, vkDestroyQueryPool, vkGetQueryPoolResults, VkCommandBuffer, VkDevice, VkDeviceSize, VkPipelineStageFlags, VkQueryPipelineStatisticFlagBits, VkQueryPool, VkQueryPoolCreateInfo, VkQueryResultFlags, VkQueryType};
#[derive(Default)]
pub struct GpuTimer {
    device: VkDevice,
    query_pool: VkQueryPool,
    timestamp_period: f32,
}

impl GpuTimer {
    pub fn new(device: VkDevice, timestamp_period: f32) -> Self {
        let create_info = VkQueryPoolCreateInfo {
            queryType: VkQueryType::TIMESTAMP,
            queryCount: 2,  // Start and end timestamp
            pipelineStatistics: VkQueryPipelineStatisticFlagBits::empty(),
            ..Default::default()
        };

        let mut query_pool = VkQueryPool::none();
        unsafe {
            vkCreateQueryPool(device, &create_info, std::ptr::null(), &mut query_pool);
        }

        Self {
            device,
            query_pool,
            timestamp_period,
        }
    }

    pub fn begin(&self, command_buffer: VkCommandBuffer) {
        unsafe {
            vkCmdResetQueryPool(command_buffer, self.query_pool, 0, 2);

            vkCmdWriteTimestamp(
                command_buffer,
                VkPipelineStageFlags::TOP_OF_PIPE_BIT,
                self.query_pool,
                0,
            );
        }
    }

    pub fn end(&self, command_buffer: VkCommandBuffer) {
        unsafe {
            vkCmdWriteTimestamp(
                command_buffer,
                VkPipelineStageFlags::TOP_OF_PIPE_BIT,
                self.query_pool,
                1,
            );
        }
    }

    pub fn get_elapsed_ms(&self) -> Option<f32> {
        let mut timestamps = [0u64; 2];

        let result = unsafe {
            vkGetQueryPoolResults(
                self.device,
                self.query_pool,
                0,
                2,
                size_of::<[u64; 2]>(),
                timestamps.as_mut_ptr() as *mut c_void,
                size_of::<u64>() as VkDeviceSize,
                VkQueryResultFlags::U64_BIT | VkQueryResultFlags::WAIT_BIT,
            )
        };

        if result.is_ok() {
            let elapsed_ticks = timestamps[1] - timestamps[0];
            let elapsed_ns = elapsed_ticks as f32 * self.timestamp_period;
            Some(elapsed_ns / 1_000_000.0)
        } else {
            None
        }
    }

    pub fn destroy(&self) {
        unsafe {
            if self.device != VkDevice::none() {
                vkDestroyQueryPool(self.device, self.query_pool, std::ptr::null());
            }
        }
    }
}

impl Drop for GpuTimer {
    fn drop(&mut self) {
        self.destroy();
    }
}
