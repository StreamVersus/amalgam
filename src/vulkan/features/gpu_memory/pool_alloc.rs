use crate::prelude::{AllocationInfo, Allocator};
use crate::vulkan::func::Vulkan;
use vulkan_raw::VkMemoryPropertyFlags;

pub struct PoolAllocation {}

impl PoolAllocator {}

impl Allocator for PoolAllocation {
    fn allocate(&mut self, flags: VkMemoryPropertyFlags, storage: S, vulkan: &Vulkan) -> AllocationInfo {
        todo!()
    }
}