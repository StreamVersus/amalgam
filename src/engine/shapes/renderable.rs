use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::utils::StagingBuffer;
use crate::vulkan::r#impl::memory::AllocationTask;
use vulkan_raw::{VkCommandBuffer, VkPipelineLayout};

pub trait Renderable {
    fn allocate(&mut self, vulkan: &Vulkan, host: &mut AllocationTask, device: &mut AllocationTask);
    fn prepare(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, staging: &mut StagingBuffer);
    fn render(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, pipeline_layout: VkPipelineLayout);
}