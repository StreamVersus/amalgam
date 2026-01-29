use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::gltf::scene::Vertex;
use crate::{null_if_none, safe_ptr};
use std::any::Any;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use vulkan_raw::{vkCmdBindIndexBuffer, vkCmdBindPipeline, vkCmdBindVertexBuffers, vkCmdPushConstants, vkCmdSetScissor, vkCmdSetViewport, vkCreateComputePipelines, vkCreateGraphicsPipelines, vkCreatePipelineCache, vkCreatePipelineLayout, vkDestroyPipeline, vkDestroyPipelineCache, vkDestroyPipelineLayout, vkGetPipelineCacheData, vkMergePipelineCaches, VkBlendFactor, VkBlendOp, VkBool32, VkBuffer, VkColorComponentFlags, VkCommandBuffer, VkCompareOp, VkComputePipelineCreateInfo, VkCullModeFlags, VkDescriptorSetLayout, VkDeviceSize, VkDynamicState, VkExtent2D, VkFormat, VkFrontFace, VkGraphicsPipelineCreateInfo, VkIndexType, VkLogicOp, VkOffset2D, VkPipeline, VkPipelineBindPoint, VkPipelineCache, VkPipelineCacheCreateInfo, VkPipelineColorBlendAttachmentState, VkPipelineColorBlendStateCreateFlags, VkPipelineColorBlendStateCreateInfo, VkPipelineCreateFlags, VkPipelineDepthStencilStateCreateFlags, VkPipelineDepthStencilStateCreateInfo, VkPipelineDynamicStateCreateFlags, VkPipelineDynamicStateCreateInfo, VkPipelineInputAssemblyStateCreateFlags, VkPipelineInputAssemblyStateCreateInfo, VkPipelineLayout, VkPipelineLayoutCreateInfo, VkPipelineMultisampleStateCreateFlags, VkPipelineMultisampleStateCreateInfo, VkPipelineRasterizationStateCreateFlags, VkPipelineRasterizationStateCreateInfo, VkPipelineShaderStageCreateFlags, VkPipelineShaderStageCreateInfo, VkPipelineTessellationStateCreateFlags, VkPipelineTessellationStateCreateInfo, VkPipelineVertexInputStateCreateFlags, VkPipelineVertexInputStateCreateInfo, VkPipelineViewportStateCreateFlags, VkPipelineViewportStateCreateInfo, VkPolygonMode, VkPrimitiveTopology, VkPushConstantRange, VkRect2D, VkRenderPass, VkSampleCountFlagBits, VkSampleMask, VkShaderModule, VkShaderStageFlagBits, VkShaderStageFlags, VkSpecializationInfo, VkSpecializationMapEntry, VkStencilOpState, VkVertexInputAttributeDescription, VkVertexInputBindingDescription, VkVertexInputRate, VkViewport};

impl Vulkan {
    #[inline]
    pub fn specify_preset_pos_tex_color() -> PipelineVertexInputStateCreateInfo {
        let binding_descriptions = vec![
            VkVertexInputBindingDescription {
                binding: 0,
                stride: size_of::<Vertex>() as u32,
                inputRate: VkVertexInputRate::VERTEX,
            },
        ];

        let attribute_descriptions = vec![
            VkVertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: VkFormat::R32G32B32_SFLOAT,
                offset: 0,
            },
            VkVertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: VkFormat::R32G32B32_SFLOAT,
                offset: 3 * size_of::<f32>() as u32,
            },
            VkVertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: VkFormat::R32G32_SFLOAT,
                offset: 6 * size_of::<f32>() as u32,
            },
        ];

        PipelineVertexInputStateCreateInfo {
            flags: Default::default(),
            vertex_binding_descriptions: binding_descriptions,
            vertex_attribute_descriptions: attribute_descriptions,
        }
    }

    #[inline]
    pub fn preset_viewport_full(screen_size: VkExtent2D) -> PipelineViewportStateCreateInfo {
        ViewportInfo {
            viewports: vec![
                VkViewport {
                    x: 0.0,
                    y: 0.0,
                    width: screen_size.width as f32,
                    height: screen_size.height as f32,
                    minDepth: 0.0,
                    maxDepth: 1.0,
                }
            ],
            scissors: vec![
                VkRect2D {
                    offset: VkOffset2D {
                        x: 0,
                        y: 0,
                    },
                    extent: screen_size,
                }
            ],
        }.into()
    }

    pub fn create_pipeline_layout(&self, descriptor_set_layouts: &[VkDescriptorSetLayout], push_constants_ranges: &[VkPushConstantRange]) -> VkPipelineLayout {
        let pipeline_layout_create_info = VkPipelineLayoutCreateInfo {
            setLayoutCount: descriptor_set_layouts.len() as u32,
            pSetLayouts: descriptor_set_layouts.as_ptr(),
            pushConstantRangeCount: push_constants_ranges.len() as u32,
            pPushConstantRanges: push_constants_ranges.as_ptr(),
            ..Default::default()
        };

        let mut pipeline_layout = VkPipelineLayout::none();
        let result = unsafe { vkCreatePipelineLayout(self.get_loaded_device().logical_device, &pipeline_layout_create_info, null_mut(), &mut pipeline_layout) };
        assert!(result.is_ok());

        pipeline_layout
    }

    pub fn create_pipeline_creation_parameters(&self, options: VkPipelineCreateFlags, shader_info: ShaderInfo,
                                               viewport_state: Option<VkPipelineViewportStateCreateInfo>, rasterization_state: Option<VkPipelineRasterizationStateCreateInfo>,
                                               multisample_state: Option<VkPipelineMultisampleStateCreateInfo>, depth_stencil_state: Option<VkPipelineDepthStencilStateCreateInfo>,
                                               color_blend_state:  Option<VkPipelineColorBlendStateCreateInfo>, dynamic_state: Option<VkPipelineDynamicStateCreateInfo>,
                                               layout: VkPipelineLayout, render_pass: VkRenderPass, subpass: u32,
                                               base_pipeline: Option<VkPipeline>, base_pipeline_index: Option<i32>) -> VkGraphicsPipelineCreateInfo {
        let flags = {
            if cfg!(debug_assertions) {
                options | VkPipelineCreateFlags::DISABLE_OPTIMIZATION_BIT
            } else {
                options
            }
        };

        VkGraphicsPipelineCreateInfo {
            flags,
            stageCount: shader_info.shader_stages.len() as u32,
            pStages: safe_ptr!(shader_info.shader_stages),
            pVertexInputState: safe_ptr!(shader_info.vertex_input_states),
            pInputAssemblyState: safe_ptr!(shader_info.input_assembly_state),
            pTessellationState: safe_ptr!(shader_info.tessellation_state),
            pViewportState: null_if_none!(viewport_state),
            pRasterizationState: null_if_none!(rasterization_state),
            pMultisampleState: null_if_none!(multisample_state),
            pDepthStencilState: null_if_none!(depth_stencil_state),
            pColorBlendState: null_if_none!(color_blend_state),
            pDynamicState: null_if_none!(dynamic_state),
            layout,
            renderPass: render_pass,
            subpass,
            basePipelineHandle: base_pipeline.unwrap_or_default(),
            basePipelineIndex: base_pipeline_index.unwrap_or(-1),
            ..Default::default()
        }
    }

    pub fn create_pipeline_cache(&self, cache_data: &[u8]) -> VkPipelineCache {
        let pipeline_cache_create_info = VkPipelineCacheCreateInfo {
            initialDataSize: cache_data.len(),
            pInitialData: cache_data.as_ptr() as *const c_void,
            ..Default::default()
        };

        let mut cache = VkPipelineCache::none();
        let result = unsafe { vkCreatePipelineCache(self.get_loaded_device().logical_device, &pipeline_cache_create_info, null_mut(), &mut cache) };
        assert!(result.is_ok());

        cache
    }

    pub fn get_data_from_pipeline_cache(&self, cache: VkPipelineCache) -> Vec<u8> {
        let mut data_size = 0;
        let result = unsafe { vkGetPipelineCacheData(self.get_loaded_device().logical_device, cache, &mut data_size, null_mut()) };
        assert!(result.is_ok());

        if data_size == 0 {
            return Vec::new();
        }

        let mut cache_data: Vec<u8> = Vec::with_capacity(data_size);
        let spare = cache_data.spare_capacity_mut();

        let result = unsafe {
            vkGetPipelineCacheData(self.get_loaded_device().logical_device, cache, &mut data_size, spare.as_mut_ptr() as *mut c_void)
        };
        assert!(result.is_ok());

        unsafe {
            cache_data.set_len(data_size);
        }

        cache_data
    }

    pub fn merge_pipeline_caches(&self, src_caches: Vec<VkPipelineCache>, dst_cache: &VkPipelineCache) {
        let result = unsafe { vkMergePipelineCaches(self.get_loaded_device().logical_device, *dst_cache, src_caches.len() as u32, src_caches.as_ptr()) };
        assert!(result.is_ok());
    }

    pub fn create_graphic_pipelines(&self, pipeline_infos: &[GraphicsPipelineCreateInfo], cache: VkPipelineCache) -> Vec<VkPipeline> {
        let mut graphic_pipelines = Vec::with_capacity(pipeline_infos.len());
        let spare = graphic_pipelines.spare_capacity_mut();

        let (pipeline_infos, _keep_alives): (Vec<VkGraphicsPipelineCreateInfo>, Vec<Vec<Box<dyn Any>>>) = pipeline_infos.iter()
            .map(|info| info.to_vulkan())
            .unzip();
        let result = unsafe { vkCreateGraphicsPipelines(self.get_loaded_device().logical_device, cache, pipeline_infos.len() as u32, pipeline_infos.as_ptr(), null_mut(), spare.as_mut_ptr() as *mut VkPipeline) };
        assert!(result.is_ok());
        unsafe {
            graphic_pipelines.set_len(pipeline_infos.len());
        }

        graphic_pipelines
    }

    pub fn create_compute_pipeline(&self, cache: Option<VkPipelineCache>, options: VkPipelineCreateFlags, stage:  VkPipelineShaderStageCreateInfo, layout: VkPipelineLayout, base_pipeline: VkPipeline) -> VkPipeline {
        let compute_pipeline_create_info = VkComputePipelineCreateInfo {
            flags: options,
            stage,
            layout,
            basePipelineHandle: base_pipeline,
            basePipelineIndex: -1,
            ..Default::default()
        };

        let mut pipeline = VkPipeline::none();
        let result = unsafe { vkCreateComputePipelines(self.get_loaded_device().logical_device, cache.unwrap_or_default(), 1, &compute_pipeline_create_info, null_mut(), &mut pipeline) };
        assert!(result.is_ok());

        pipeline
    }

    pub fn bind_vertex_buffers(&self, command_buffer: VkCommandBuffer, offset: u32,  parameters: Vec<VertexBufferParameters>) {
        let mut buffers: Vec<VkBuffer> = Vec::with_capacity(parameters.len());
        let mut offsets: Vec<VkDeviceSize> = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            buffers.push(parameter.buffer);
            offsets.push(parameter.offset);
        }

        unsafe { vkCmdBindVertexBuffers(command_buffer, offset, buffers.len() as u32, buffers.as_ptr(), offsets.as_ptr()) ; }
    }

    pub fn bind_index_buffer(&self, command_buffer: VkCommandBuffer, buffer: VkBuffer, offset: u64, index_type: VkIndexType) {
        unsafe { vkCmdBindIndexBuffer(command_buffer, buffer, offset, index_type); }
    }
    /// # Safety
    /// Check passed data pointers
    pub unsafe fn set_push_constants(&self, command_buffer: VkCommandBuffer, layout: VkPipelineLayout, stage_flags: VkShaderStageFlags, offset: u32, size: u32, data: *const c_void) {
        unsafe { vkCmdPushConstants(command_buffer, layout, stage_flags, offset, size, data); }
    }

    pub fn set_viewport(&self, command_buffer: VkCommandBuffer, offset: u32, viewports: &[VkViewport]) {
        unsafe { vkCmdSetViewport(command_buffer, offset, viewports.len() as u32, viewports.as_ptr()); }
    }

    pub fn set_scissors(&self, command_buffer: VkCommandBuffer, offset: u32, scissors: &[VkRect2D]) {
        unsafe { vkCmdSetScissor(command_buffer, offset, scissors.len() as u32, scissors.as_ptr()); }
    }

    pub fn bind_pipeline(&self, command_buffer: VkCommandBuffer, pipeline_type: VkPipelineBindPoint, pipeline: VkPipeline) {
        unsafe { vkCmdBindPipeline(command_buffer, pipeline_type, pipeline) };
    }

    fn destroy_pipeline(&self, pipeline: VkPipeline) {
        unsafe { vkDestroyPipeline(self.get_loaded_device().logical_device, pipeline, null_mut()) };
    }

    fn destroy_pipeline_cache(&self, pipeline_cache: VkPipelineCache) {
        unsafe { vkDestroyPipelineCache(self.get_loaded_device().logical_device, pipeline_cache, null_mut()) }
    }

    fn destroy_pipeline_layout(&self, pipeline_layout: VkPipelineLayout) {
        if let Ok(loaded_device) = self.safe_get_loaded_device() {
            unsafe { vkDestroyPipelineLayout(loaded_device.logical_device, pipeline_layout, null_mut()) }
        }
    }
}

impl Destructible for VkPipeline {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_pipeline(*self);
    }
}

impl Destructible for VkPipelineCache {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_pipeline_cache(*self);
    }
}

impl Destructible for VkPipelineLayout {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_pipeline_layout(*self);
    }
}

pub struct ShaderInfo {
    pub shader_stages: Vec<VkPipelineShaderStageCreateInfo>,
    pub vertex_input_states: Vec<VkPipelineVertexInputStateCreateInfo>,
    pub input_assembly_state: Vec<VkPipelineInputAssemblyStateCreateInfo>,
    pub tessellation_state: Vec<VkPipelineTessellationStateCreateInfo>,
}

pub struct VertexBufferParameters {
    pub buffer: VkBuffer,
    pub offset: VkDeviceSize,
}

pub struct ViewportInfo {
    pub viewports: Vec<VkViewport>,
    pub scissors: Vec<VkRect2D>,
}

impl From<ViewportInfo> for PipelineViewportStateCreateInfo {
    fn from(value: ViewportInfo) -> Self {
        PipelineViewportStateCreateInfo {
            flags: Default::default(),
            viewports: value.viewports,
            scissors: value.scissors,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct GraphicsPipelineCreateInfo {
    pub flags: VkPipelineCreateFlags,
    pub stages: Vec<PipelineShaderStageCreateInfo>,
    pub vertex_input_state: Option<PipelineVertexInputStateCreateInfo>,
    pub input_assembly_state: Option<PipelineInputAssemblyStateCreateInfo>,
    pub tessellation_state: Option<PipelineTessellationStateCreateInfo>,
    pub viewport_state: Option<PipelineViewportStateCreateInfo>,
    pub rasterization_state: Option<PipelineRasterizationStateCreateInfo>,
    pub multisample_state: Option<PipelineMultisampleStateCreateInfo>,
    pub depth_stencil_state: Option<PipelineDepthStencilStateCreateInfo>,
    pub color_blend_state: Option<PipelineColorBlendStateCreateInfo>,
    pub dynamic_state: Option<PipelineDynamicStateCreateInfo>,
    pub layout: VkPipelineLayout,
    pub render_pass: VkRenderPass,
    pub subpass: u32,
    pub base_pipeline_handle: VkPipeline,
    pub base_pipeline_index: i32,
}

#[derive(Clone, Debug)]
pub struct PipelineShaderStageCreateInfo {
    pub flags: VkPipelineShaderStageCreateFlags,
    pub stage: VkShaderStageFlagBits,
    pub module: VkShaderModule,
    pub name: &'static str,
    pub specialization_info: Option<SpecializationInfo>,
}

#[derive(Clone, Debug)]
pub struct SpecializationInfo {
    pub map_entries: Vec<VkSpecializationMapEntry>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct PipelineVertexInputStateCreateInfo {
    pub flags: VkPipelineVertexInputStateCreateFlags,
    pub vertex_binding_descriptions: Vec<VkVertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<VkVertexInputAttributeDescription>,
}

#[derive(Clone, Debug)]
pub struct PipelineInputAssemblyStateCreateInfo {
    pub flags: VkPipelineInputAssemblyStateCreateFlags,
    pub topology: VkPrimitiveTopology,
    pub primitive_restart_enable: VkBool32,
}

#[derive(Clone, Debug)]
pub struct PipelineTessellationStateCreateInfo {
    pub flags: VkPipelineTessellationStateCreateFlags,
    pub patch_control_points: u32,
}

#[derive(Clone, Debug)]
pub struct PipelineViewportStateCreateInfo {
    pub flags: VkPipelineViewportStateCreateFlags,
    pub viewports: Vec<VkViewport>,
    pub scissors: Vec<VkRect2D>,
}

#[derive(Clone, Debug)]
pub struct PipelineRasterizationStateCreateInfo {
    pub flags: VkPipelineRasterizationStateCreateFlags,
    pub depth_clamp_enable: VkBool32,
    pub rasterizer_discard_enable: VkBool32,
    pub polygon_mode: VkPolygonMode,
    pub cull_mode: VkCullModeFlags,
    pub front_face: VkFrontFace,
    pub depth_bias_enable: VkBool32,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub line_width: f32,
}

#[derive(Clone, Debug)]
pub struct PipelineMultisampleStateCreateInfo {
    pub flags: VkPipelineMultisampleStateCreateFlags,
    pub rasterization_samples: VkSampleCountFlagBits,
    pub sample_shading_enable: VkBool32,
    pub min_sample_shading: f32,
    pub sample_mask: Vec<VkSampleMask>,
    pub alpha_to_coverage_enable: VkBool32,
    pub alpha_to_one_enable: VkBool32,
}

#[derive(Clone, Debug)]
pub struct PipelineDepthStencilStateCreateInfo {
    pub flags: VkPipelineDepthStencilStateCreateFlags,
    pub depth_test_enable: VkBool32,
    pub depth_write_enable: VkBool32,
    pub depth_compare_op: VkCompareOp,
    pub depth_bounds_test_enable: VkBool32,
    pub stencil_test_enable: VkBool32,
    pub front: VkStencilOpState,
    pub back: VkStencilOpState,
    pub min_depth_bounds: f32,
    pub max_depth_bounds: f32,
}

#[derive(Clone, Debug)]
pub struct PipelineColorBlendStateCreateInfo {
    pub flags: VkPipelineColorBlendStateCreateFlags,
    pub logic_op_enable: VkBool32,
    pub logic_op: VkLogicOp,
    pub attachments: Vec<PipelineColorBlendAttachmentState>,
    pub blend_constants: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct PipelineColorBlendAttachmentState {
    pub blend_enable: VkBool32,
    pub src_color_blend_factor: VkBlendFactor,
    pub dst_color_blend_factor: VkBlendFactor,
    pub color_blend_op: VkBlendOp,
    pub src_alpha_blend_factor: VkBlendFactor,
    pub dst_alpha_blend_factor: VkBlendFactor,
    pub alpha_blend_op: VkBlendOp,
    pub color_write_mask: VkColorComponentFlags,
}

#[derive(Clone, Debug)]
pub struct PipelineDynamicStateCreateInfo {
    pub flags: VkPipelineDynamicStateCreateFlags,
    pub dynamic_states: Vec<VkDynamicState>,
}

// Conversion implementations
impl GraphicsPipelineCreateInfo {

    pub fn to_vulkan(&self) -> (VkGraphicsPipelineCreateInfo, Vec<Box<dyn Any>>) {
        // We need to keep intermediate data alive
        let mut keep_alive: Vec<Box<dyn Any>> = Vec::with_capacity(19);

        // Convert stages
        let stages: Vec<_> = self.stages.iter()
            .map(|stage| stage.to_vulkan(&mut keep_alive))
            .collect();
        let stages = Box::new(stages);

        let vertex_ptr: *const _;
        match &self.vertex_input_state {
            Some(vertex_input) => {
            let boxed = Box::new(vertex_input.to_vulkan(&mut keep_alive));
            vertex_ptr = boxed.as_ref();
            keep_alive.push(boxed);
            },
            None => vertex_ptr = null()
        };

        let input_assembly_ptr: *const _;
        match &self.input_assembly_state {
            Some(input_assembly) => {
                let boxed = Box::new(input_assembly.to_vulkan(&mut keep_alive));
                input_assembly_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => input_assembly_ptr = null()
        };

        let tessellation_ptr: *const _;
        match &self.tessellation_state {
            Some(tessellation_input) => {
                let boxed = Box::new(tessellation_input.to_vulkan(&mut keep_alive));
                tessellation_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => tessellation_ptr = null()
        };

        let viewport_ptr: *const _;
        match &self.viewport_state {
            Some(viewport_input) => {
                let boxed = Box::new(viewport_input.to_vulkan(&mut keep_alive));
                viewport_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => viewport_ptr = null()
        };

        let rasterization_ptr: *const _;
        match &self.rasterization_state {
            Some(rasterization_input) => {
                let boxed = Box::new(rasterization_input.to_vulkan(&mut keep_alive));
                rasterization_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => rasterization_ptr = null()
        };

        let multisample_ptr: *const _;
        match &self.multisample_state {
            Some(multisample_input) => {
                let boxed = Box::new(multisample_input.to_vulkan(&mut keep_alive));
                multisample_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => multisample_ptr = null()
        };

        let depth_stencil_ptr: *const _;
        match &self.depth_stencil_state {
            Some(depth_stencil_input) => {
                let boxed = Box::new(depth_stencil_input.to_vulkan(&mut keep_alive));
                depth_stencil_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => depth_stencil_ptr = null()
        };

        let color_blend_ptr: *const _;
        match &self.color_blend_state {
            Some(color_blend) => {
                let boxed = Box::new(color_blend.to_vulkan(&mut keep_alive));
                color_blend_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => color_blend_ptr = null()
        };

        let dynamic_ptr: *const _;
        match &self.dynamic_state {
            Some(dynamic_input) => {
                let boxed = Box::new(dynamic_input.to_vulkan(&mut keep_alive));
                dynamic_ptr = boxed.as_ref();
                keep_alive.push(boxed);
            },
            None => dynamic_ptr = null()
        };

        let info = VkGraphicsPipelineCreateInfo {
            flags: self.flags,
            stageCount: stages.len() as u32,
            pStages: stages.as_ptr(),
            pVertexInputState: vertex_ptr,
            pInputAssemblyState: input_assembly_ptr,
            pTessellationState: tessellation_ptr,
            pViewportState: viewport_ptr,
            pRasterizationState: rasterization_ptr,
            pMultisampleState: multisample_ptr,
            pDepthStencilState: depth_stencil_ptr,
            pColorBlendState: color_blend_ptr,
            pDynamicState: dynamic_ptr,
            layout: self.layout,
            renderPass: self.render_pass,
            subpass: self.subpass,
            basePipelineHandle: self.base_pipeline_handle,
            basePipelineIndex: self.base_pipeline_index,
            ..Default::default()
        };
        keep_alive.push(stages);

        (info, keep_alive)
    }
}

impl PipelineShaderStageCreateInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkPipelineShaderStageCreateInfo {
        // Convert name to C string
        let c_name = Box::new(std::ffi::CString::new(self.name).unwrap());
        let name_ptr = c_name.as_ptr();
        keep_alive.push(c_name);
        
        let mut spec_ptr: *const VkSpecializationInfo = null();
        if let Some(spec) = &self.specialization_info {
            let boxed = Box::new(spec.to_vulkan(keep_alive));
            spec_ptr = boxed.as_ref();
            keep_alive.push(boxed); 
        };

        VkPipelineShaderStageCreateInfo {
            flags: self.flags,
            stage: self.stage,
            module: self.module,
            pName: name_ptr,
            pSpecializationInfo: spec_ptr,
            ..Default::default()
        }
    }
}

impl SpecializationInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkSpecializationInfo {
        let map_entries_box = Box::new(self.map_entries.clone());
        let data_box = Box::new(self.data.clone());

        let info = VkSpecializationInfo {
            mapEntryCount: map_entries_box.len() as u32,
            pMapEntries: map_entries_box.as_ptr(),
            dataSize: data_box.len(),
            pData: data_box.as_ptr() as *const c_void,
        };

        keep_alive.push(map_entries_box);
        keep_alive.push(data_box);

        info
    }
}

impl PipelineVertexInputStateCreateInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkPipelineVertexInputStateCreateInfo {
        let binding_descriptions = Box::new(self.vertex_binding_descriptions.clone());
        let attribute_descriptions = Box::new(self.vertex_attribute_descriptions.clone());

        let info = VkPipelineVertexInputStateCreateInfo {
            flags: self.flags,
            vertexBindingDescriptionCount: binding_descriptions.len() as u32,
            pVertexBindingDescriptions: binding_descriptions.as_ptr(),
            vertexAttributeDescriptionCount: attribute_descriptions.len() as u32,
            pVertexAttributeDescriptions: attribute_descriptions.as_ptr(),
            ..Default::default()
        };

        keep_alive.push(binding_descriptions);
        keep_alive.push(attribute_descriptions);

        info
    }
}

impl PipelineInputAssemblyStateCreateInfo {
    pub fn to_vulkan(&self, _keep_alive: &mut [Box<dyn Any>]) -> VkPipelineInputAssemblyStateCreateInfo {
        VkPipelineInputAssemblyStateCreateInfo {
            flags: self.flags,
            topology: self.topology,
            primitiveRestartEnable: self.primitive_restart_enable,
            ..Default::default()
        }
    }
}

impl PipelineViewportStateCreateInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkPipelineViewportStateCreateInfo {
        let viewports = Box::new(self.viewports.clone());
        let scissors = Box::new(self.scissors.clone());

        let info = VkPipelineViewportStateCreateInfo {
            flags: self.flags,
            viewportCount: self.viewports.len() as u32,
            pViewports: if viewports.is_empty() { null() } else { viewports.as_ptr() },
            scissorCount: self.scissors.len() as u32,
            pScissors: if scissors.is_empty() { null() } else { scissors.as_ptr() },
            ..Default::default()
        };

        keep_alive.push(viewports);
        keep_alive.push(scissors);

        info
    }
}

impl PipelineColorBlendAttachmentState {
    pub fn to_vulkan(&self, _keep_alive: &mut [Box<dyn Any>]) -> VkPipelineColorBlendAttachmentState {
        VkPipelineColorBlendAttachmentState {
            blendEnable: self.blend_enable,
            srcColorBlendFactor: self.src_color_blend_factor,
            dstColorBlendFactor: self.dst_color_blend_factor,
            colorBlendOp: self.color_blend_op,
            srcAlphaBlendFactor: self.src_alpha_blend_factor,
            dstAlphaBlendFactor: self.dst_alpha_blend_factor,
            alphaBlendOp: self.alpha_blend_op,
            colorWriteMask: self.color_write_mask,
        }
    }
}

impl PipelineColorBlendStateCreateInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkPipelineColorBlendStateCreateInfo {
        let attachments: Vec<VkPipelineColorBlendAttachmentState> = self.attachments.iter().map(|attachment| attachment.to_vulkan(keep_alive)).collect();
        let attachments = Box::new(attachments);
        let info = VkPipelineColorBlendStateCreateInfo {
            flags: self.flags,
            logicOpEnable: self.logic_op_enable,
            logicOp: self.logic_op,
            attachmentCount: attachments.len() as u32,
            pAttachments: attachments.as_ptr(),
            blendConstants: self.blend_constants,
            ..Default::default()
        };

        keep_alive.push(attachments);
        info
    }
}

impl PipelineDynamicStateCreateInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkPipelineDynamicStateCreateInfo {
        let boxed = Box::new(self.dynamic_states.clone());

        let info = VkPipelineDynamicStateCreateInfo {
            flags: self.flags,
            dynamicStateCount: boxed.len() as u32,
            pDynamicStates: boxed.as_ptr(),
            ..Default::default()
        };

        keep_alive.push(boxed);
        info
    }
}

impl PipelineTessellationStateCreateInfo {
    pub fn to_vulkan(&self, _keep_alive: &mut [Box<dyn Any>]) -> VkPipelineTessellationStateCreateInfo {
        VkPipelineTessellationStateCreateInfo {
            flags: self.flags,
            patchControlPoints: self.patch_control_points,
            ..Default::default()
        }
    }
}

impl PipelineRasterizationStateCreateInfo {
    pub fn to_vulkan(&self, _keep_alive: &mut [Box<dyn Any>]) -> VkPipelineRasterizationStateCreateInfo {
        VkPipelineRasterizationStateCreateInfo {
            flags: self.flags,
            depthClampEnable: self.depth_clamp_enable,
            rasterizerDiscardEnable: self.rasterizer_discard_enable,
            polygonMode: self.polygon_mode,
            cullMode: self.cull_mode,
            frontFace: self.front_face,
            depthBiasEnable: self.depth_bias_enable,
            depthBiasConstantFactor: self.depth_bias_constant_factor,
            depthBiasClamp: self.depth_bias_clamp,
            depthBiasSlopeFactor: self.depth_bias_slope_factor,
            lineWidth: self.line_width,
            ..Default::default()
        }
    }
}

impl PipelineMultisampleStateCreateInfo {
    pub fn to_vulkan(&self, keep_alive: &mut Vec<Box<dyn Any>>) -> VkPipelineMultisampleStateCreateInfo {
        let boxed = Box::new(self.sample_mask.clone());

        let info = VkPipelineMultisampleStateCreateInfo {
            flags: self.flags,
            rasterizationSamples: self.rasterization_samples,
            sampleShadingEnable: self.sample_shading_enable,
            minSampleShading: self.min_sample_shading,
            pSampleMask: if boxed.is_empty() {
                null()
            } else {
                boxed.as_ptr()
            },
            alphaToCoverageEnable: self.alpha_to_coverage_enable,
            alphaToOneEnable: self.alpha_to_one_enable,
            ..Default::default()
        };

        keep_alive.push(boxed);
        info
    }
}

impl PipelineDepthStencilStateCreateInfo {
    pub fn to_vulkan(&self, _keep_alive: &mut [Box<dyn Any>]) -> VkPipelineDepthStencilStateCreateInfo {
        VkPipelineDepthStencilStateCreateInfo {
            flags: self.flags,
            depthTestEnable: self.depth_test_enable,
            depthWriteEnable: self.depth_write_enable,
            depthCompareOp: self.depth_compare_op,
            depthBoundsTestEnable: self.depth_bounds_test_enable,
            stencilTestEnable: self.stencil_test_enable,
            front: self.front,
            back: self.back,
            minDepthBounds: self.min_depth_bounds,
            maxDepthBounds: self.max_depth_bounds,
            ..Default::default()
        }
    }
}

unsafe impl Send for GraphicsPipelineCreateInfo {}
unsafe impl Sync for GraphicsPipelineCreateInfo {}
unsafe impl Send for PipelineShaderStageCreateInfo {}
unsafe impl Sync for PipelineShaderStageCreateInfo {}
unsafe impl Send for SpecializationInfo {}
unsafe impl Sync for SpecializationInfo {}
unsafe impl Send for PipelineVertexInputStateCreateInfo {}
unsafe impl Sync for PipelineVertexInputStateCreateInfo {}
unsafe impl Send for PipelineInputAssemblyStateCreateInfo {}
unsafe impl Sync for PipelineInputAssemblyStateCreateInfo {}
unsafe impl Send for PipelineTessellationStateCreateInfo {}
unsafe impl Sync for PipelineTessellationStateCreateInfo {}
unsafe impl Send for PipelineViewportStateCreateInfo {}
unsafe impl Sync for PipelineViewportStateCreateInfo {}
unsafe impl Send for PipelineRasterizationStateCreateInfo {}
unsafe impl Sync for PipelineRasterizationStateCreateInfo {}
unsafe impl Send for PipelineMultisampleStateCreateInfo {}
unsafe impl Sync for PipelineMultisampleStateCreateInfo {}
unsafe impl Send for PipelineDepthStencilStateCreateInfo {}
unsafe impl Sync for PipelineDepthStencilStateCreateInfo {}
unsafe impl Send for PipelineColorBlendStateCreateInfo {}
unsafe impl Sync for PipelineColorBlendStateCreateInfo {}
unsafe impl Send for PipelineDynamicStateCreateInfo {}
unsafe impl Sync for PipelineDynamicStateCreateInfo {}
