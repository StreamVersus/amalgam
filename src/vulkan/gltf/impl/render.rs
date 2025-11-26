use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::gltf::scene::Scene;
use crate::vulkan::gltf::utils::IndirectParameters;
use crate::vulkan::r#impl::command_buffer::RecordingInfo;
use crate::vulkan::r#impl::image::ImageTransition;
use crate::vulkan::r#impl::memory::AllocationTask;
use crate::vulkan::utils::BufferUsage;
use vulkan_raw::{vkCmdDrawIndexedIndirect, VkAccessFlags, VkBufferCopy, VkBufferImageCopy, VkCommandBuffer, VkCommandBufferLevel, VkCommandBufferUsageFlags, VkCommandPoolCreateFlags, VkDeviceSize, VkFence, VkImageAspectFlags, VkImageLayout, VkImageSubresourceLayers, VkIndexType, VkPipelineBindPoint, VkPipelineLayout, VkPipelineStageFlags, VK_QUEUE_FAMILY_IGNORED};

impl Scene {
    pub fn prepare(&mut self, vulkan: &Vulkan) {
        let mut max_staging_size = self.idx_size + self.parameters_size;
        for image in &self.texture_images {
            max_staging_size += image.size as u64;
        }

        let staging_buffer = vulkan.create_buffer(max_staging_size, BufferUsage::preset_staging()).unwrap();
        let info = AllocationTask::host_cached().add_allocatable(staging_buffer).allocate_all(vulkan);
        let staging_info = info.pull_buffer_info(&staging_buffer);
        let staging_ptr = vulkan.map_memory(&staging_info);

        let command_pool = vulkan.create_command_pool(vulkan.get_loaded_device().queue_info[0].family_index, VkCommandPoolCreateFlags::empty());
        let one_time_command_buffer = vulkan.alloc_command_buffers(command_pool, VkCommandBufferLevel::PRIMARY, 1)[0];
        // prepare staging buffer
        let mut image_offsets: Vec<VkDeviceSize> = Vec::with_capacity(self.texture_images.len());
        let mut current_offset = 0u64;

        unsafe {
            // Copy indices
            Vulkan::copy_info(staging_ptr, self.indices.as_ptr(), self.indices.len());
            current_offset += self.idx_size;

            // Copy parameters
            Vulkan::copy_info(staging_ptr.add(current_offset as usize), self.parameters.as_ptr(), self.parameters.len());
            current_offset += self.parameters_size;

            // Copy images
            for image in &self.texture_images {
                Vulkan::copy_info(staging_ptr.add(current_offset as usize), image.data.as_ptr(), image.size);
                image_offsets.push(current_offset);
                current_offset += image.size as u64;
            }
        }
        vulkan.flush_memory(&[staging_info]);

        vulkan.start_recording(one_time_command_buffer, VkCommandBufferUsageFlags::ONE_TIME_SUBMIT_BIT, RecordingInfo {
            renderPass: Default::default(),
            subpass: 0,
            framebuffer: Default::default(),
            occlusionQueryEnable: false,
            queryFlags: Default::default(),
            pipelineStatistics: Default::default(),
        });

        self.vbo.sync_buffer(vulkan, one_time_command_buffer);

        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: 0,
            dstOffset: 0,
            size: (self.indices.len() * size_of::<u16>()) as VkDeviceSize,
        }], one_time_command_buffer, staging_buffer, *self.idx.get());

        vulkan.buffer_to_buffer(vec![VkBufferCopy {
            srcOffset: self.idx_size,
            dstOffset: 0,
            size: self.parameters_size,
        }], one_time_command_buffer, staging_buffer, *self.indirect_buffer.get());

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

        staging_buffer.destroy(vulkan);
        info.get_all_memory_objects().into_iter().for_each(|memory_object| { memory_object.destroy(vulkan); });

        vulkan.free_buffers(command_pool, &[one_time_command_buffer]);
        command_pool.destroy(vulkan);

        self.texture_images.iter_mut().for_each(|image| {
            image.data.clear()
        });
    }

    pub fn render_scene(&self, vulkan: &Vulkan, command_buffer: VkCommandBuffer, pipeline_layout: VkPipelineLayout) {
        self.vbo.bind(vulkan, command_buffer);

        vulkan.bind_index_buffer(command_buffer, *self.idx.get(), 0, VkIndexType::UINT16);
        vulkan.bind_descriptor_sets(command_buffer, VkPipelineBindPoint::GRAPHICS, pipeline_layout, 0, &self.descriptor_sets, &[]);

        unsafe { vkCmdDrawIndexedIndirect(command_buffer, *self.indirect_buffer.get(), 0, self.parameters.len() as u32, size_of::<IndirectParameters>() as u32) };
    }
}