use crate::safe_ptr;
use crate::vulkan::func::{Destructible, Vulkan};
use std::any::Any;
use std::ptr::null_mut;
use vulkan_raw::{vkAllocateDescriptorSets, vkCmdBindDescriptorSets, vkCreateDescriptorPool, vkCreateDescriptorSetLayout, vkDestroyDescriptorPool, vkDestroyDescriptorSetLayout, vkFreeDescriptorSets, vkResetDescriptorPool, vkUpdateDescriptorSets, VkBufferView, VkCommandBuffer, VkCopyDescriptorSet, VkDescriptorBufferInfo, VkDescriptorImageInfo, VkDescriptorPool, VkDescriptorPoolCreateFlags, VkDescriptorPoolCreateInfo, VkDescriptorPoolResetFlagBits, VkDescriptorPoolSize, VkDescriptorSet, VkDescriptorSetAllocateInfo, VkDescriptorSetLayout, VkDescriptorSetLayoutBinding, VkDescriptorSetLayoutCreateInfo, VkDescriptorType, VkPipelineBindPoint, VkPipelineLayout, VkResult, VkWriteDescriptorSet};

impl Vulkan {
    pub fn create_descriptor_set_layout(&self, bindings: &[VkDescriptorSetLayoutBinding]) -> VkDescriptorSetLayout {
        let descriptor_set_layout_create_info = VkDescriptorSetLayoutCreateInfo {
            bindingCount: bindings.len() as u32,
            pBindings: bindings.as_ptr(),
            ..Default::default()
        };
        
        let mut descriptor_set_layout = VkDescriptorSetLayout::none();
        let result = unsafe { vkCreateDescriptorSetLayout(self.get_loaded_device().logical_device, &descriptor_set_layout_create_info, null_mut(), &mut descriptor_set_layout) };
        assert_eq!(result, VkResult::SUCCESS);
        
        descriptor_set_layout
    }
    
    pub fn create_descriptor_pool(&self, descriptor_types: &[VkDescriptorPoolSize], max_sets_count: u32, free_individual_sets: bool) -> VkDescriptorPool {
        let descriptor_pool_create_info = VkDescriptorPoolCreateInfo {
            flags: {
                if free_individual_sets {
                    VkDescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET_BIT
                } else { 
                    VkDescriptorPoolCreateFlags::empty()
                }
            },
            maxSets: max_sets_count,
            poolSizeCount: descriptor_types.len() as u32,
            pPoolSizes: descriptor_types.as_ptr(),
            ..Default::default()
        };
        
        let mut descriptor_pool = VkDescriptorPool::none();
        let result = unsafe { vkCreateDescriptorPool(self.get_loaded_device().logical_device, &descriptor_pool_create_info, null_mut(), &mut descriptor_pool) };
        assert_eq!(result, VkResult::SUCCESS);
        
        descriptor_pool
    }

    pub fn allocate_descriptor_sets(&self, descriptor_pool: VkDescriptorPool, descriptor_set_layouts: &[VkDescriptorSetLayout]) -> Vec<VkDescriptorSet> {
        let descriptor_set_allocate_info = VkDescriptorSetAllocateInfo {
            descriptorPool: descriptor_pool,
            descriptorSetCount: descriptor_set_layouts.len() as u32,
            pSetLayouts: descriptor_set_layouts.as_ptr(),
            ..Default::default()
        };

        let mut descriptor_sets: Vec<VkDescriptorSet> = Vec::with_capacity(descriptor_set_layouts.len());
        let spare = descriptor_sets.spare_capacity_mut();
        let result = unsafe {
            vkAllocateDescriptorSets(self.get_loaded_device().logical_device, &descriptor_set_allocate_info, spare.as_mut_ptr() as *mut VkDescriptorSet)
        };
        assert_eq!(result, VkResult::SUCCESS);

        unsafe {
            descriptor_sets.set_len(descriptor_set_layouts.len());
        }
        descriptor_sets
    }

    pub fn update_descriptor_sets(&self, image_descriptor_infos: Vec<ImageDescriptorInfo>, buffer_descriptor_infos: Vec<BufferDescriptorInfo>, texel_buffer_descriptor_infos: Vec<TexelBufferDescriptorInfo>, copy_descriptor_infos: Vec<CopyDescriptorInfo>) {
        let write_size = image_descriptor_infos.len() + buffer_descriptor_infos.len() + texel_buffer_descriptor_infos.len();
        let mut write_vec: Vec<VkWriteDescriptorSet> = Vec::with_capacity(write_size);
        let mut copy_vec: Vec<VkCopyDescriptorSet> = Vec::with_capacity(copy_descriptor_infos.len());

        let mut keep_alives: Vec<Box<dyn Any>> = Vec::with_capacity(2);

        image_descriptor_infos.into_iter().for_each(|info| { write_vec.push(info.to_vulkan(&mut keep_alives)) });
        buffer_descriptor_infos.into_iter().for_each(|info| { write_vec.push(info.to_vulkan(&mut keep_alives)) });
        texel_buffer_descriptor_infos.into_iter().for_each(|info| { write_vec.push(info.to_vulkan(&mut keep_alives)) });

        copy_descriptor_infos.into_iter().for_each(|info| { copy_vec.push(info.to_vulkan()) });
        unsafe {
            vkUpdateDescriptorSets(
                self.get_loaded_device().logical_device,
                write_vec.len() as u32, safe_ptr!(write_vec),
                copy_vec.len() as u32, safe_ptr!(copy_vec)
            )
        };
    }

    pub fn bind_descriptor_sets(&self, command_buffer: VkCommandBuffer, pipeline_bind_point: VkPipelineBindPoint, layout: VkPipelineLayout,
                                first_set: u32, descriptor_sets: &[VkDescriptorSet], dynamic_offsets: &[u32]) {
        unsafe { vkCmdBindDescriptorSets(command_buffer, pipeline_bind_point, layout, first_set, descriptor_sets.len() as u32, descriptor_sets.as_ptr(), dynamic_offsets.len() as u32, dynamic_offsets.as_ptr()) }
    }

    pub fn free_descriptor_sets(&mut self, descriptor_pool: VkDescriptorPool, descriptor_sets: Vec<VkDescriptorSet>) {
        let result = unsafe { vkFreeDescriptorSets(self.get_loaded_device().logical_device, descriptor_pool, descriptor_sets.len() as u32, safe_ptr!(descriptor_sets)) };
        assert_eq!(result, VkResult::SUCCESS);
    }

    pub fn reset_descriptor_pool(&self, descriptor_pool: VkDescriptorPool) {
        let result = unsafe { vkResetDescriptorPool(self.get_loaded_device().logical_device, descriptor_pool, VkDescriptorPoolResetFlagBits::empty()) };
        assert_eq!(result, VkResult::SUCCESS);
    }

    fn destroy_descriptor_pool(&self, descriptor_pool: VkDescriptorPool) {
        unsafe { vkDestroyDescriptorPool(self.get_loaded_device().logical_device, descriptor_pool, null_mut()) };
    }
    
    fn destroy_descriptor_set_layout(&self, descriptor_set_layout: VkDescriptorSetLayout) {
        unsafe { vkDestroyDescriptorSetLayout(self.get_loaded_device().logical_device, descriptor_set_layout, null_mut()) };
    }
}

pub struct DescriptorSetInfo {
    pub descriptor_set: VkDescriptorSet,
    pub descriptor_binding: u32,
    pub array_element: u32,
}

pub struct ImageDescriptorInfo {
    pub target_descriptor: DescriptorSetInfo,
    pub target_descriptor_type: VkDescriptorType,
    pub image_infos: Vec<VkDescriptorImageInfo>,
}

impl ImageDescriptorInfo {
    pub fn to_vulkan(self, keep_alives: &mut Vec<Box<dyn Any>>) -> VkWriteDescriptorSet {
        let image_infos = Box::new(self.image_infos);
        let info = VkWriteDescriptorSet {
            dstSet: self.target_descriptor.descriptor_set,
            dstBinding: self.target_descriptor.descriptor_binding,
            dstArrayElement: self.target_descriptor.array_element,
            descriptorCount: image_infos.len() as u32,
            descriptorType: self.target_descriptor_type,
            pImageInfo: image_infos.as_ptr(),
            ..Default::default()
        };
        keep_alives.push(image_infos);
        info
    }
}

pub struct BufferDescriptorInfo {
    pub target_descriptor: DescriptorSetInfo,
    pub target_descriptor_type: VkDescriptorType,
    pub buffer_infos: Vec<VkDescriptorBufferInfo>,
}

impl BufferDescriptorInfo {
    pub fn to_vulkan(self, keep_alives: &mut Vec<Box<dyn Any>>) -> VkWriteDescriptorSet {
        let buffer_infos = Box::new(self.buffer_infos);

        let info = VkWriteDescriptorSet {
            dstSet: self.target_descriptor.descriptor_set,
            dstBinding: self.target_descriptor.descriptor_binding,
            dstArrayElement: self.target_descriptor.array_element,
            descriptorCount: buffer_infos.len() as u32,
            descriptorType: self.target_descriptor_type,
            pBufferInfo: buffer_infos.as_ptr(),
            ..Default::default()
        };
        keep_alives.push(buffer_infos);

        info
    }
}

pub struct TexelBufferDescriptorInfo {
    target_descriptor: DescriptorSetInfo,
    target_descriptor_type: VkDescriptorType,
    texel_buffer_views: Vec<VkBufferView>,
}

impl TexelBufferDescriptorInfo {
    pub fn to_vulkan(self, keep_alives: &mut Vec<Box<dyn Any>>) -> VkWriteDescriptorSet {
        let texel_buffer_views = Box::new(self.texel_buffer_views);
        let info = VkWriteDescriptorSet {
            dstSet: self.target_descriptor.descriptor_set,
            dstBinding: self.target_descriptor.descriptor_binding,
            dstArrayElement: self.target_descriptor.array_element,
            descriptorCount: texel_buffer_views.len() as u32,
            descriptorType: self.target_descriptor_type,
            pTexelBufferView: texel_buffer_views.as_ptr(),
            ..Default::default()
        };

        keep_alives.push(texel_buffer_views);
        info
    }
}

pub struct CopyDescriptorInfo {
    target_descriptor: DescriptorSetInfo,
    source_descriptor: DescriptorSetInfo,
    descriptor_count: u32,
}

impl CopyDescriptorInfo {
    pub fn to_vulkan(self) -> VkCopyDescriptorSet {
        VkCopyDescriptorSet {
            srcSet: self.source_descriptor.descriptor_set,
            srcBinding: self.source_descriptor.descriptor_binding,
            srcArrayElement: self.source_descriptor.array_element,
            dstSet: self.target_descriptor.descriptor_set,
            dstBinding: self.target_descriptor.descriptor_binding,
            dstArrayElement: self.target_descriptor.array_element,
            descriptorCount: self.descriptor_count,
            ..Default::default()
        }
    }
}

impl Destructible for VkDescriptorPool {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_descriptor_pool(*self);
    }
}

impl Destructible for VkDescriptorSetLayout {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_descriptor_set_layout(*self);
    }
}