use crate::safe_ptr;
use crate::vulkan::func::{Destructible, Vulkan};
use std::any::Any;
use std::ptr::null;
use std::ptr::null_mut;
use vulkan_raw::{vkCmdBeginRenderPass, vkCmdClearAttachments, vkCmdEndRenderPass, vkCmdNextSubpass, vkCreateFramebuffer, vkCreateRenderPass, vkDestroyFramebuffer, vkDestroyRenderPass, VkAccessFlags, VkAttachmentDescription, VkAttachmentLoadOp, VkAttachmentReference, VkAttachmentStoreOp, VkClearAttachment, VkClearRect, VkClearValue, VkCommandBuffer, VkFormat, VkFramebuffer, VkFramebufferCreateInfo, VkImageLayout, VkImageView, VkPipelineBindPoint, VkPipelineStageFlags, VkRect2D, VkRenderPass, VkRenderPassBeginInfo, VkRenderPassCreateInfo, VkResult, VkSampleCountFlags, VkSubpassContents, VkSubpassDependency, VkSubpassDescription, VK_SUBPASS_EXTERNAL};

impl Vulkan {
    pub fn create_render_pass(&self, attachment_description: Vec<VkAttachmentDescription>, subpass_parameters: Vec<SubpassParameters>, subpass_dependencies: Vec<VkSubpassDependency>) -> VkRenderPass {
        let mut subpass_descriptions: Vec<VkSubpassDescription> = Vec::with_capacity(subpass_dependencies.len());
        let mut keep_alives: Vec<Box<dyn Any>> = Vec::with_capacity(subpass_descriptions.len());
        subpass_parameters.into_iter()
            .map(|x| x.to_vulkan())
            .for_each(|x| {
                subpass_descriptions.push(x.0);
                keep_alives.extend(x.1);
            });

        let create_info = VkRenderPassCreateInfo {
            attachmentCount: attachment_description.len() as u32,
            pAttachments: safe_ptr!(attachment_description),
            subpassCount: subpass_descriptions.len() as u32,
            pSubpasses: safe_ptr!(subpass_descriptions),
            dependencyCount: subpass_dependencies.len() as u32,
            pDependencies: safe_ptr!(subpass_dependencies),
            ..Default::default()
        };

        let mut render_pass = VkRenderPass::none();
        let result = unsafe { vkCreateRenderPass(self.get_loaded_device().logical_device, &create_info, null_mut(), &mut render_pass) };
        assert_eq!(result, VkResult::SUCCESS);

        render_pass
    }

    pub fn create_framebuffer(&self, render_pass: VkRenderPass, attachments: &[VkImageView], width: u32, height: u32, layers: u32) -> VkFramebuffer {
        let create_info = VkFramebufferCreateInfo {
            renderPass: render_pass,
            attachmentCount: attachments.len() as u32,
            pAttachments: safe_ptr!(attachments),
            width,
            height,
            layers,
            ..Default::default()
        };

        let mut framebuffer = VkFramebuffer::none();
        let result = unsafe { vkCreateFramebuffer(self.get_loaded_device().logical_device, &create_info, null_mut(), &mut framebuffer) };
        assert_eq!(result, VkResult::SUCCESS);

        framebuffer
    }

    pub fn preset_renderpass_color_depth(&self, samples: VkSampleCountFlags, format: VkFormat) -> VkRenderPass {
        let color_layout = if samples == VkSampleCountFlags::SC_1_BIT {
            VkImageLayout::PRESENT_SRC_KHR
        } else {
            VkImageLayout::COLOR_ATTACHMENT_OPTIMAL
        };
        let mut attachment_descriptions = vec![
            VkAttachmentDescription{
                format,
                samples,
                loadOp: VkAttachmentLoadOp::CLEAR,
                storeOp: VkAttachmentStoreOp::STORE,
                stencilLoadOp: VkAttachmentLoadOp::DONT_CARE,
                stencilStoreOp: VkAttachmentStoreOp::DONT_CARE,
                initialLayout: VkImageLayout::UNDEFINED,
                finalLayout: color_layout,
                ..Default::default()
            },
            VkAttachmentDescription{
                format: VkFormat::D32_SFLOAT,
                samples,
                loadOp: VkAttachmentLoadOp::CLEAR,
                storeOp: VkAttachmentStoreOp::STORE,
                stencilLoadOp: VkAttachmentLoadOp::DONT_CARE,
                stencilStoreOp: VkAttachmentStoreOp::DONT_CARE,
                initialLayout: VkImageLayout::UNDEFINED,
                finalLayout: VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                ..Default::default()
            },
        ];
        if samples != VkSampleCountFlags::SC_1_BIT {
            attachment_descriptions.push(VkAttachmentDescription {
                format,
                samples: VkSampleCountFlags::SC_1_BIT,
                loadOp: VkAttachmentLoadOp::DONT_CARE,
                storeOp: VkAttachmentStoreOp::STORE,
                stencilLoadOp: VkAttachmentLoadOp::DONT_CARE,
                stencilStoreOp: VkAttachmentStoreOp::DONT_CARE,
                initialLayout: VkImageLayout::UNDEFINED,
                finalLayout: VkImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            });
        }

        let depth_stencil_attachment = VkAttachmentReference {
            attachment: 1,
            layout: VkImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };
        let mut resolve_attachments: Vec<VkAttachmentReference> = vec![];
        if samples != VkSampleCountFlags::SC_1_BIT {
            resolve_attachments.push(VkAttachmentReference {
                attachment: 2,
                layout: VkImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            });
        }
        let subpass_descriptions = vec![
            SubpassParameters {
                pipeline_type: VkPipelineBindPoint::GRAPHICS,
                input_attachments: vec![],
                color_attachments: vec![
                    VkAttachmentReference {
                        attachment: 0,
                        layout: VkImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }
                ],
                resolve_attachments,
                depth_stencil_attachment: Some(depth_stencil_attachment),
                preserve_attachments: vec![],
            }
        ];

        let subpass_dependencies = vec![
            VkSubpassDependency {
                srcSubpass: 0,
                dstSubpass: VK_SUBPASS_EXTERNAL,
                srcStageMask: VkPipelineStageFlags::COLOR_ATTACHMENT_OUTPUT_BIT,
                dstStageMask: VkPipelineStageFlags::FRAGMENT_SHADER_BIT,
                srcAccessMask: VkAccessFlags::COLOR_ATTACHMENT_WRITE_BIT,
                dstAccessMask: VkAccessFlags::SHADER_READ_BIT,
                dependencyFlags: Default::default(),
            }
        ];
        self.create_render_pass(attachment_descriptions, subpass_descriptions, subpass_dependencies)
    }

    pub fn begin_render_pass(&self, command_buffer: VkCommandBuffer, render_pass: VkRenderPass, framebuffer: VkFramebuffer, render_area: VkRect2D, clear_values: &[VkClearValue], subpass_contents: VkSubpassContents) {
        let render_pass_begin_info = VkRenderPassBeginInfo {
            renderPass: render_pass,
            framebuffer,
            renderArea: render_area,
            clearValueCount: clear_values.len() as u32,
            pClearValues: if clear_values.is_empty() {
                null_mut()
            } else {
                clear_values.as_ptr()
            },
            ..Default::default()
        };
        unsafe { vkCmdBeginRenderPass(command_buffer, &render_pass_begin_info, subpass_contents) };
    }

    pub fn next_subpass(&self, command_buffer: VkCommandBuffer, subpass_contents: VkSubpassContents) {
        unsafe { vkCmdNextSubpass(command_buffer, subpass_contents); }
    }

    pub fn end_render_pass(&self, command_buffer: VkCommandBuffer) {
        unsafe { vkCmdEndRenderPass(command_buffer); }
    }

    fn destroy_framebuffer(&self, framebuffer: VkFramebuffer) {
        unsafe { vkDestroyFramebuffer(self.get_loaded_device().logical_device, framebuffer, null()); };
    }

    fn destroy_render_pass(&self, render_pass: VkRenderPass) {
        unsafe { vkDestroyRenderPass(self.get_loaded_device().logical_device, render_pass, null()); };
    }

    pub fn clear_attachments(&self, command_buffer: VkCommandBuffer, attachments: Vec<VkClearAttachment>, rects: Vec<VkClearRect>) {
        unsafe { vkCmdClearAttachments(command_buffer, attachments.len() as u32, attachments.as_ptr(), rects.len() as u32, rects.as_ptr()) }
    }
}

pub struct SubpassParameters {
    pipeline_type: VkPipelineBindPoint,
    input_attachments: Vec<VkAttachmentReference>,
    color_attachments: Vec<VkAttachmentReference>,
    resolve_attachments: Vec<VkAttachmentReference>,
    depth_stencil_attachment: Option<VkAttachmentReference>,
    preserve_attachments: Vec<u32>,
}

impl SubpassParameters {
    fn to_vulkan(self) -> (VkSubpassDescription, Vec<Box<dyn Any>>) {
        let mut keep_alives: Vec<Box<dyn Any>> = Vec::with_capacity(5);
        let input_attachments = Box::new(self.input_attachments);
        let color_attachments = Box::new(self.color_attachments);
        let resolve_attachments = Box::new(self.resolve_attachments);
        let depth_stencil_attachment_ptr: *const VkAttachmentReference ={
            match self.depth_stencil_attachment {
                Some(attachment) => {
                    let boxed = Box::new(attachment);
                    let ptr = boxed.as_ref() as *const dyn Any as *const VkAttachmentReference;
                    keep_alives.push(boxed);
                    ptr
                },
                None => null_mut(),
            }
        };
        let preserve_attachments = Box::new(self.preserve_attachments);

        let info = VkSubpassDescription {
            pipelineBindPoint: self.pipeline_type,
            inputAttachmentCount: input_attachments.len() as u32,
            pInputAttachments: safe_ptr!(input_attachments),
            colorAttachmentCount: color_attachments.len() as u32,
            pColorAttachments: safe_ptr!(color_attachments),
            pResolveAttachments: safe_ptr!(resolve_attachments),
            pDepthStencilAttachment: depth_stencil_attachment_ptr,
            preserveAttachmentCount: preserve_attachments.len() as u32,
            pPreserveAttachments: safe_ptr!(preserve_attachments),
            ..Default::default()
        };

        keep_alives.push(input_attachments);
        keep_alives.push(color_attachments);
        keep_alives.push(resolve_attachments);
        keep_alives.push(preserve_attachments);
        (info, keep_alives)
    }
}

impl Destructible for VkFramebuffer {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_framebuffer(*self);
    }
}

impl Destructible for VkRenderPass {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_render_pass(*self);
    }
}