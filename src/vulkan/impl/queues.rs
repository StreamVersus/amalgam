use crate::vulkan::func::Vulkan;
use std::ptr::null_mut;
use vulkan_raw::{vkGetDeviceQueue, vkGetPhysicalDeviceQueueFamilyProperties, vkQueueWaitIdle, VkPhysicalDevice, VkQueue, VkQueueFamilyProperties, VkQueueFlagBits, VkQueueFlags, VkResult};

impl Vulkan {
    pub fn get_queue_families(&self, device: VkPhysicalDevice) -> Vec<VkQueueFamilyProperties> {
        let mut queue_family_count: u32 = 0;
        unsafe {
            vkGetPhysicalDeviceQueueFamilyProperties(device, &mut queue_family_count, null_mut());
        }
        assert_ne!(queue_family_count, 0);

        let mut queue_families: Vec<VkQueueFamilyProperties> = Vec::with_capacity(queue_family_count as usize);
        let spare = queue_families.spare_capacity_mut();
        unsafe {
            vkGetPhysicalDeviceQueueFamilyProperties(device, &mut queue_family_count, spare.as_mut_ptr() as *mut VkQueueFamilyProperties);
        }
        assert_ne!(queue_family_count, 0);

        unsafe {
            queue_families.set_len(queue_family_count as usize);
        }
        queue_families
    }
    
    pub fn get_desired_family(&self, device: VkPhysicalDevice, desired_family: VkQueueFlags) -> Vec<u32> {
        let queue_families = self.get_queue_families(device);
        let mut ret_vec: Vec<u32> = vec![];

        for (i, family) in queue_families.iter().enumerate() {
            if family.queueCount > 0 && family.queueFlags.intersects(desired_family) {
                ret_vec.push(i as u32);
            }
        }
        
        if ret_vec.is_empty() {
            panic!("Unable to find suitable queue families");
        };
        ret_vec
    }
    
    pub fn get_queues(&self) -> Vec<VkQueue> {
        let queue_info = &self.get_loaded_device().queue_info;
        let device = self.get_loaded_device().logical_device;
        let mut ret_vec: Vec<VkQueue> = Vec::with_capacity(queue_info.len());
        
        queue_info.iter().for_each(|queue| { 
            let family_index = queue.family_index;
            for i in 0..queue.priorities.len() {
                let mut queue = VkQueue::none();
                unsafe {
                    vkGetDeviceQueue(device, family_index, i as u32, &mut queue);
                }
                
                ret_vec.push(queue);
            }
        });
        
        ret_vec
    }
    
    pub fn build_desired_queue_info(&self, device: VkPhysicalDevice) -> Vec<QueueInfo> {
        let desired_graphical_families = self.get_desired_family(device, VkQueueFlagBits::GRAPHICS_BIT);
        let mut ret_vec: Vec<QueueInfo> = vec![];

        for x in &desired_graphical_families {
            ret_vec.push(QueueInfo {
                family_index: *x,
                priorities: vec![1.0],
            });
        }
        if ret_vec.len() == 0 {
            ret_vec.push(QueueInfo {
                family_index: desired_graphical_families[0],
                priorities: vec![1.0],
            });
        }
        
        ret_vec
    }
    
    pub fn wait_for_queue(&self, queue: VkQueue) -> bool {
        unsafe { vkQueueWaitIdle(queue) == VkResult::SUCCESS }
    }
}

#[derive(Debug)]
#[derive(Clone)]
pub struct QueueInfo {
    pub family_index: u32,
    pub priorities: Vec<f32>
}