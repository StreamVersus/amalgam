use crate::vulkan::func::{Destructible, Vulkan};
use std::ffi::CString;
use std::ptr::null_mut;
use vulkan_raw::{vkCreateShaderModule, vkDestroyShaderModule, VkPipelineShaderStageCreateInfo, VkResult, VkShaderModule, VkShaderModuleCreateInfo, VkShaderStageFlags, VkSpecializationInfo};

impl Vulkan {
    pub fn create_shader_module(&self, shader_bytecode: &[u8]) -> VkShaderModule {
        let shader_module_create_info = VkShaderModuleCreateInfo {
            codeSize: shader_bytecode.len(),
            pCode: shader_bytecode.as_ptr() as *const u32,
            ..Default::default()
        };

        let mut shader_module = VkShaderModule::none();
        let result = unsafe { vkCreateShaderModule(self.get_loaded_device().logical_device, &shader_module_create_info, null_mut(), &mut shader_module) };
        assert_eq!(result, VkResult::SUCCESS);

        shader_module
    }
    
    fn destroy_shader_module(&self, shader_module: VkShaderModule) {
        unsafe { vkDestroyShaderModule(self.get_loaded_device().logical_device, shader_module, null_mut()) };
    }
}

impl Destructible for VkShaderModule {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_shader_module(*self);
    }
}

pub struct ShaderStageParameters {
    pub shader_stage: VkShaderStageFlags,
    pub shader_module: VkShaderModule,
    pub entrypoint: CString,
    pub specialization_info: VkSpecializationInfo,
}

impl Into<VkPipelineShaderStageCreateInfo> for ShaderStageParameters {
    fn into(self) -> VkPipelineShaderStageCreateInfo {
        VkPipelineShaderStageCreateInfo {
            stage: self.shader_stage,
            module: self.shader_module,
            pName: self.entrypoint.as_ptr(),
            pSpecializationInfo: &self.specialization_info,
            ..Default::default()
        }
    }
}