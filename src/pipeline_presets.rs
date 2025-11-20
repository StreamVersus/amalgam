use crate::vulkan::func::{bool_to_vkbool, Vulkan};
use crate::vulkan::r#impl::pipelines::{GraphicsPipelineCreateInfo, PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo, PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineTessellationStateCreateInfo};
use crate::vulkan::shaders::{GRAPHIC_FRAG, GRAPHIC_VERT};
use vulkan_raw::{VkBlendFactor, VkBlendOp, VkBool32, VkColorComponentFlags, VkCompareOp, VkCullModeFlags, VkDescriptorSetLayout, VkDynamicState, VkExtent2D, VkFrontFace, VkLogicOp, VkPipelineLayout, VkPipelineShaderStageCreateFlags, VkPolygonMode, VkPrimitiveTopology, VkRenderPass, VkSampleCountFlagBits, VkSampleCountFlags, VkShaderStageFlags, VkStencilOp, VkStencilOpState};

pub fn preset_graphic_pipeline(vulkan: &Vulkan, width: u32, height: u32, render_pass: VkRenderPass, subpass: u32, descriptor_set_layouts: &[VkDescriptorSetLayout]) -> (GraphicsPipelineCreateInfo, VkPipelineLayout) {
    let push_constant_ranges = &[

    ];

    let layout = vulkan.create_pipeline_layout(descriptor_set_layouts, push_constant_ranges);

    let vertex_shader_module = vulkan.create_shader_module(GRAPHIC_VERT);
    let frag_shader_module = vulkan.create_shader_module(GRAPHIC_FRAG);

    (GraphicsPipelineCreateInfo {
        flags: Default::default(),
        stages: vec![
            PipelineShaderStageCreateInfo {
                flags: VkPipelineShaderStageCreateFlags::empty(),
                stage: VkShaderStageFlags::VERTEX_BIT,
                module: vertex_shader_module,
                name: "main",
                specialization_info: None,
            },
            PipelineShaderStageCreateInfo {
                flags: VkPipelineShaderStageCreateFlags::empty(),
                stage: VkShaderStageFlags::FRAGMENT_BIT,
                module: frag_shader_module,
                name: "main",
                specialization_info: None,
            },
        ],
        vertex_input_state: Some(Vulkan::specify_preset_pos_tex_color()),
        input_assembly_state: Some(
            PipelineInputAssemblyStateCreateInfo {
                flags: Default::default(),
                topology: VkPrimitiveTopology::TRIANGLE_LIST,
                primitive_restart_enable: bool_to_vkbool(false),
            }
        ),
        tessellation_state: Some (PipelineTessellationStateCreateInfo {
            flags: Default::default(),
            patch_control_points: 1,
        }),
        viewport_state: Some(Vulkan::preset_viewport_full(VkExtent2D { width, height })),
        rasterization_state: Some (PipelineRasterizationStateCreateInfo {
            flags: Default::default(),
            depth_clamp_enable: bool_to_vkbool(false),
            rasterizer_discard_enable: bool_to_vkbool(false),
            polygon_mode: VkPolygonMode::FILL,
            cull_mode: VkCullModeFlags::NONE,
            front_face: VkFrontFace::COUNTER_CLOCKWISE,
            depth_bias_enable: bool_to_vkbool(false),
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        }),
        multisample_state: Some(PipelineMultisampleStateCreateInfo {
            flags: Default::default(),
            rasterization_samples: VkSampleCountFlags::SC_1_BIT,
            sample_shading_enable: Default::default(),
            min_sample_shading: 0.0,
            sample_mask: vec![],
            alpha_to_coverage_enable: Default::default(),
            alpha_to_one_enable: Default::default(),
        }),
        depth_stencil_state: Some(PipelineDepthStencilStateCreateInfo {
            flags: Default::default(),
            depth_test_enable: VkBool32::TRUE,
            depth_write_enable: VkBool32::TRUE,
            depth_compare_op: VkCompareOp::LESS_OR_EQUAL,
            depth_bounds_test_enable: VkBool32::FALSE,
            stencil_test_enable: VkBool32::FALSE,
            front: VkStencilOpState {
                failOp: VkStencilOp::KEEP,
                passOp: VkStencilOp::KEEP,
                depthFailOp: VkStencilOp::KEEP,
                compareOp: VkCompareOp::ALWAYS,
                compareMask: 0,
                writeMask: 0,
                reference: 0,
            },
            back: VkStencilOpState {
                failOp: VkStencilOp::KEEP,
                passOp: VkStencilOp::KEEP,
                depthFailOp: VkStencilOp::KEEP,
                compareOp: VkCompareOp::ALWAYS,
                compareMask: 0,
                writeMask: 0,
                reference: 0,
            },
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }),
        color_blend_state: Some(PipelineColorBlendStateCreateInfo {
            flags: Default::default(),
            logic_op_enable: VkBool32::FALSE,
            logic_op: VkLogicOp::COPY,
            attachments: vec![
                PipelineColorBlendAttachmentState {
                    blend_enable: VkBool32::FALSE,
                    src_color_blend_factor: VkBlendFactor::ZERO,
                    dst_color_blend_factor: VkBlendFactor::ZERO,
                    color_blend_op: VkBlendOp::ADD,
                    src_alpha_blend_factor: VkBlendFactor::ZERO,
                    dst_alpha_blend_factor: VkBlendFactor::ZERO,
                    alpha_blend_op: VkBlendOp::ADD,
                    color_write_mask:
                        VkColorComponentFlags::R_BIT |
                        VkColorComponentFlags::G_BIT |
                        VkColorComponentFlags::B_BIT |
                        VkColorComponentFlags::A_BIT,
                }
            ],
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        }),
        dynamic_state: Some(PipelineDynamicStateCreateInfo {
            flags: Default::default(),
            dynamic_states: vec![
                VkDynamicState::VIEWPORT,
                VkDynamicState::SCISSOR,
            ],
        }),
        layout,
        render_pass,
        subpass,
        base_pipeline_handle: Default::default(),
        base_pipeline_index: -1,
    }, layout)
}

pub fn preset_multisample(main_pipeline: GraphicsPipelineCreateInfo, samples: VkSampleCountFlags, cap: VkSampleCountFlags) -> GraphicsPipelineCreateInfo {
    GraphicsPipelineCreateInfo {
        flags: main_pipeline.flags,
        stages: main_pipeline.stages,
        vertex_input_state: main_pipeline.vertex_input_state,
        input_assembly_state: main_pipeline.input_assembly_state,
        tessellation_state: main_pipeline.tessellation_state,
        viewport_state: main_pipeline.viewport_state,
        rasterization_state: main_pipeline.rasterization_state,
        multisample_state: Some(PipelineMultisampleStateCreateInfo {
            flags: Default::default(),
            rasterization_samples: resolve_highest_multisampling(samples, cap),
            sample_shading_enable: VkBool32::FALSE, // TODO
            min_sample_shading: 0.4,
            sample_mask: vec![],
            alpha_to_coverage_enable: Default::default(),
            alpha_to_one_enable: Default::default(),
        }),
        depth_stencil_state: main_pipeline.depth_stencil_state,
        color_blend_state: main_pipeline.color_blend_state,
        dynamic_state: main_pipeline.dynamic_state,
        layout: main_pipeline.layout,
        render_pass: main_pipeline.render_pass,
        subpass: main_pipeline.subpass,
        base_pipeline_handle: main_pipeline.base_pipeline_handle,
        base_pipeline_index: main_pipeline.base_pipeline_index,
    }
}

const SAMPLE_COUNTS: &[VkSampleCountFlags] = &[
    VkSampleCountFlags::SC_2_BIT,
    VkSampleCountFlags::SC_4_BIT,
    VkSampleCountFlags::SC_8_BIT,
    VkSampleCountFlags::SC_16_BIT,
    VkSampleCountFlags::SC_32_BIT,
    VkSampleCountFlags::SC_64_BIT,
];

fn resolve_supported_multisampling(supported_samples: VkSampleCountFlagBits, cap: VkSampleCountFlagBits) -> Vec<VkSampleCountFlags> {
    let mut ret_vec = Vec::new();
    for &sample in SAMPLE_COUNTS {
        if supported_samples.contains(sample) {
            ret_vec.push(sample);
            if cap == sample {
                break;
            }
        }
    }
    ret_vec
}

pub fn resolve_highest_multisampling(supported_samples: VkSampleCountFlagBits, cap: VkSampleCountFlagBits) -> VkSampleCountFlags {
    if cap == VkSampleCountFlags::SC_1_BIT {
        return VkSampleCountFlags::SC_1_BIT;
    }

    for &sample in SAMPLE_COUNTS.iter().rev() {
        if sample <= cap && supported_samples.contains(sample) {
            return sample;
        }
    }

    VkSampleCountFlags::SC_1_BIT
}