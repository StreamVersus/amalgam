use crate::vulkan::func::{bool_to_vkbool, Destructible, Vulkan};
use std::ptr::null;
use vulkan_raw::{vkCreateFence, vkCreateSemaphore, vkDestroyFence, vkDestroySemaphore, vkDeviceWaitIdle, vkResetFences, vkWaitForFences, VkFence, VkFenceCreateFlagBits, VkFenceCreateFlags, VkFenceCreateInfo, VkResult, VkSemaphore, VkSemaphoreCreateInfo};

impl Vulkan {
    pub fn create_semaphore(&self) -> VkSemaphore {
        let mut semaphore = VkSemaphore::none();
        let result = unsafe { vkCreateSemaphore(self.get_loaded_device().logical_device, &VkSemaphoreCreateInfo::default(), null(), &mut semaphore) };
        assert_eq!(result, VkResult::SUCCESS);
        
        semaphore
    }
    
    pub fn destroy_semaphore(&self, semaphore: VkSemaphore) {
        if semaphore != VkSemaphore::none() {
            unsafe { vkDestroySemaphore(self.get_loaded_device().logical_device, semaphore, null()) };
        }
    }
    
    pub fn create_fence(&self, signaled: bool) -> VkFence {
        let fence_create_info = VkFenceCreateInfo {
            flags: {
                if signaled {
                    VkFenceCreateFlagBits::SIGNALED_BIT
                } else {
                    VkFenceCreateFlags::empty()
                }
            },
            ..Default::default()
        };
        
        let mut fence = VkFence::none();
        let result = unsafe { vkCreateFence(self.get_loaded_device().logical_device, &fence_create_info, null(), &mut fence) };
        assert_eq!(result, VkResult::SUCCESS);
        
        fence
    }
    
    pub fn wait_for_fences(&self, fences: &[VkFence], wait_for_all: bool, timeout: u64) {
        if fences.len() != 0 {
            let result = unsafe { vkWaitForFences(self.get_loaded_device().logical_device, fences.len() as u32, fences.as_ptr(), bool_to_vkbool(wait_for_all), timeout) };
            if result != VkResult::SUCCESS {
                eprintln!("Failed to wait for fences with {} fences", fences.len());
            }
        }
    }
    
    pub fn reset_fences(&self, fences: &[VkFence]) {
        if fences.len() != 0 {
            let result = unsafe { vkResetFences(self.get_loaded_device().logical_device, fences.len() as u32, fences.as_ptr()) };
            if result != VkResult::SUCCESS {
                eprintln!("Failed to reset fences with {} fences", fences.len());
            }
        }
    }

    pub fn destroy_fence(&self, fence: VkFence) {
        if fence != VkFence::none() {
            unsafe { vkDestroyFence(self.get_loaded_device().logical_device, fence, null()) };
        }
    }

    pub fn device_wait(&self) {
        unsafe { vkDeviceWaitIdle(self.get_loaded_device().logical_device) };
    }
}

impl Destructible for VkFence {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_fence(*self);
    }
}

impl Destructible for VkSemaphore {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_semaphore(*self);
    }
}
