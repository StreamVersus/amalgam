use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::gltf::scene::{MaterialID, Scene};
use crate::vulkan::gltf::utils::{IndirectParameters, StagingBuffer};
use crate::vulkan::r#impl::command_buffer::RecordingInfo;
use crate::vulkan::r#impl::image::ImageTransition;
use ultraviolet::Mat4;
use vulkan_raw::{vkCmdDrawIndexedIndirect, VkAccessFlags, VkBufferCopy, VkBufferImageCopy, VkCommandBuffer, VkCommandBufferLevel, VkCommandBufferUsageFlags, VkCommandPoolCreateFlags, VkDeviceSize, VkFence, VkImageAspectFlags, VkImageLayout, VkImageSubresourceLayers, VkIndexType, VkPipelineBindPoint, VkPipelineLayout, VkPipelineStageFlags, VK_QUEUE_FAMILY_IGNORED};

impl Scene {
    pub fn prepare(&mut self, vulkan: &Vulkan, staging: &mut StagingBuffer) {
        let mut max_staging_size = (self.idx.size() + self.parameters.size()) as u64;

        // Add SSBO sizes
        max_staging_size += (self.model_matrices.len() * size_of::<Mat4>()) as u64;
        max_staging_size += (self.material_ranges.len() * size_of::<MaterialID>()) as u64;
        for image in &self.texture_images {
            max_staging_size += image.size as u64;
        }

        let staging_buffer = staging.pull(max_staging_size, vulkan);
        let staging_info = staging.info();
        let staging_ptr = vulkan.map_memory(staging_info);

        let command_pool = vulkan.create_command_pool(vulkan.get_loaded_device().queue_info[0].family_index, VkCommandPoolCreateFlags::empty());
        let one_time_command_buffer = vulkan.alloc_command_buffers(command_pool, VkCommandBufferLevel::PRIMARY, 1)[0];

        // prepare staging buffer
        let mut image_offsets: Vec<VkDeviceSize> = Vec::with_capacity(self.texture_images.len());
        let mut current_offset = 0usize;

        unsafe {
            // Copy indices
            Vulkan::copy_info(staging_ptr, self.indices.as_ptr(), self.indices.len());
            current_offset += self.indices.len() * size_of::<u16>();

            // Copy parameters
            Vulkan::copy_info(staging_ptr.add(current_offset), self.parameters.as_ptr(), self.parameters.len());
            current_offset += self.parameters.size();

            // Copy model matrices
            Vulkan::copy_info(staging_ptr.add(current_offset), self.model_matrices.as_ptr(), self.model_matrices.len());
            current_offset += self.model_matrices.len() * size_of::<Mat4>();

            // Copy material ranges
            Vulkan::copy_info(staging_ptr.add(current_offset), self.material_ranges.as_ptr(), self.material_ranges.len());
            current_offset += self.material_ranges.len() * size_of::<MaterialID>();

            // Copy images
            for image in &self.texture_images {
                Vulkan::copy_info(staging_ptr.add(current_offset), image.data.as_ptr(), image.size);
                image_offsets.push(current_offset as VkDeviceSize);
                current_offset += image.size;
            }
        }
        vulkan.flush_memory(&[*staging_info]);

        vulkan.start_recording(one_time_command_buffer, VkCommandBufferUsageFlags::ONE_TIME_SUBMIT_BIT, RecordingInfo {
            renderPass: Default::default(),
            subpass: 0,
            framebuffer: Default::default(),
            occlusionQueryEnable: false,
            queryFlags: Default::default(),
            pipelineStatistics: Default::default(),
        });

        self.vbo.sync_buffer(vulkan, one_time_command_buffer);

        let mut offset: VkDeviceSize = 0;
        // Copy indices
        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: offset,
            dstOffset: 0,
            size: self.idx.size() as VkDeviceSize,
        }], one_time_command_buffer, staging_buffer, *self.idx.get());
        offset += self.idx.size() as u64;

        // Copy parameters
        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: offset,
            dstOffset: 0,
            size: self.parameters.size() as VkDeviceSize,
        }], one_time_command_buffer, staging_buffer, *self.indirect_buffer.get());
        offset += self.parameters.size() as u64;

        // Copy model ssbo
        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: offset,
            dstOffset: 0,
            size: (self.model_matrices.len() * size_of::<Mat4>()) as VkDeviceSize,
        }], one_time_command_buffer, staging_buffer, *self.model_ssbo.get());
        offset += (self.model_matrices.len() * size_of::<Mat4>()) as VkDeviceSize;

        // Copy material ssbo
        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: offset,
            dstOffset: 0,
            size: (self.material_ranges.len() * size_of::<MaterialID>()) as VkDeviceSize,
        }], one_time_command_buffer, staging_buffer, *self.material_ssbo.get());
        //offset += (self.material_ranges.len() * size_of::<MaterialID>()) as VkDeviceSize;

        // Transition and copy images
        let transitions = self.texture_images.iter().map(|image| {
            ImageTransition {
                image: *image.image.get(),
                current_access: VkAccessFlags::NONE,
                new_access: VkAccessFlags::NONE,
                current_layout: VkImageLayout::UNDEFINED,
                new_layout: VkImageLayout::TRANSFER_DST_OPTIMAL,
                current_queue_family: VK_QUEUE_FAMILY_IGNORED,
                new_queue_family: VK_QUEUE_FAMILY_IGNORED,
                aspect: VkImageAspectFlags::COLOR_BIT,
            }
        }).collect::<Vec<_>>();

        vulkan.transition_images(transitions, one_time_command_buffer, VkPipelineStageFlags::TOP_OF_PIPE_BIT, VkPipelineStageFlags::TRANSFER_BIT);

        let transitions = self.texture_images.iter().zip(image_offsets.iter()).map(|(image, &buffer_offset)| {
            vulkan.buffer_to_image(vec![
                VkBufferImageCopy {
                    bufferOffset: buffer_offset,
                    bufferRowLength: 0,
                    bufferImageHeight: 0,
                    imageSubresource: VkImageSubresourceLayers {
                        aspectMask: VkImageAspectFlags::COLOR_BIT,
                        mipLevel: 0,
                        baseArrayLayer: 0,
                        layerCount: 1,
                    },
                    imageOffset: Default::default(),
                    imageExtent: image.extent,
                }
            ], one_time_command_buffer, staging_buffer, *image.image.get(), VkImageLayout::TRANSFER_DST_OPTIMAL);

            ImageTransition {
                image: *image.image.get(),
                current_access: VkAccessFlags::NONE,
                new_access: VkAccessFlags::NONE,
                current_layout: VkImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: VkImageLayout::SHADER_READ_ONLY_OPTIMAL,
                current_queue_family: VK_QUEUE_FAMILY_IGNORED,
                new_queue_family: VK_QUEUE_FAMILY_IGNORED,
                aspect: VkImageAspectFlags::COLOR_BIT,
            }
        }).collect::<Vec<_>>();

        vulkan.transition_images(transitions, one_time_command_buffer, VkPipelineStageFlags::TOP_OF_PIPE_BIT, VkPipelineStageFlags::FRAGMENT_SHADER_BIT);

        vulkan.end_recording(one_time_command_buffer);
        vulkan.submit_buffer(vulkan.get_queues()[0], VkFence::none(), &[one_time_command_buffer], &[], &[]);
        vulkan.wait_for_queue(vulkan.get_queues()[0]);

        vulkan.free_buffers(command_pool, &[one_time_command_buffer]);
        command_pool.destroy(vulkan);

        self.texture_images.iter_mut().for_each(|image| {
            image.data.clear()
        });
    }

    pub fn render_scene(&self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, pipeline_layout: VkPipelineLayout) {
        self.vbo.bind(vulkan, command_buffer);

        vulkan.bind_index_buffer(command_buffer, *self.idx.get(), 0, VkIndexType::UINT16);
        vulkan.bind_descriptor_sets(command_buffer, VkPipelineBindPoint::GRAPHICS, pipeline_layout, 0, &self.descriptors.descriptor_sets, &[]);

        unsafe { vkCmdDrawIndexedIndirect(command_buffer, *self.indirect_buffer.get(), 0, self.parameters.len() as u32, size_of::<IndirectParameters>() as u32) };
    }
}