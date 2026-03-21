use crate::vulkan::func::Vulkan;
use egui::ClippedPrimitive;
use std::time::Instant;

use crate::prelude::*;

#[derive(Default)]
pub struct FastRenderer {
    render_pass: VkDestroy<VkRenderPass>,
    //vbo: VBO,
    //idx: 
    //pipeline: VkPipeline,
}

impl FastRenderer {
    pub fn new(vulkan: &Vulkan, format: VkFormat) -> FastRenderer {
        let render_pass = vulkan.preset_renderpass_color(format, VkImageLayout::COLOR_ATTACHMENT_OPTIMAL, VkImageLayout::PRESENT_SRC_KHR);

        let render_pass = VkDestroy::new(render_pass, vulkan);
        FastRenderer {
            render_pass,
            //pipeline,
        }
    }

    pub fn render_primitives(&self, vulkan: &Vulkan,
                             frame_resource: PerFrameResource,
                             image_resource: PerImageResource,
                             command_buffer: VkCommandBuffer, extent: VkExtent2D,
                             primitives: Vec<ClippedPrimitive>) -> f32 {
        let instant = Instant::now();

        let clear_values = vec![
            VkClearValue { color: VkClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } },
            VkClearValue { depthStencil: VkClearDepthStencilValue { depth: 1.0, stencil: 0 } },
        ];
        vulkan.begin_render_pass(frame_resource.command_buffer(), *self.render_pass, image_resource.framebuffer(),
                                 VkRect2D { offset: Default::default(), extent }, clear_values.as_slice(), VkSubpassContents::INLINE);

        //Write into buffer
        // for clipped in &primitives {
        //     if let Primitive::Mesh(mesh) = &clipped.primitive {
        //         // Upload vertices
        //         vertex_buffer.write(&mesh.vertices);
        // 
        //         // Upload indices
        //         index_buffer.write(&mesh.indices);
        //     }
        // }

        instant.elapsed().as_secs_f32()
    }
}