use crate::prelude::{AllocationInfo, Allocator};
use crate::vulkan::func::Vulkan;
use vulkan_raw::VkMemoryPropertyFlags;

pub struct PoolAllocator {
    tlsf: TLSF,
}

impl Allocator for PoolAllocator {
    fn allocate(&mut self, flags: VkMemoryPropertyFlags, storage: S, vulkan: &Vulkan) -> AllocationInfo {
        todo!()
    }
}

struct TLSF {

}