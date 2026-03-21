use crate::prelude::*;
use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::utils::StagingBuffer;

pub trait Renderable {
    fn allocate(&mut self, vulkan: &Vulkan, host: &mut BatchedStorage, device: &mut BatchedStorage);
    fn prepare(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, staging: &mut StagingBuffer);
    fn render(&mut self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, pipeline_layout: VkPipelineLayout);
}