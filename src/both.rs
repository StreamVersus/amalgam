use crate::engine::camera::Camera;
use crate::engine::pipelines::create_pipelines_multithreaded;
use crate::engine::{PerFrameResource, PerImageResource, Settings};
use crate::pipeline_presets::{preset_graphic_pipeline, preset_multisample, resolve_highest_multisampling};
use crate::vulkan::func::Vulkan;
use crate::vulkan::gltf::scene::Scene;
use crate::vulkan::r#impl::command_buffer::{RecordingInfo, WaitSemaphoreInfo};
use crate::vulkan::r#impl::memory::VkDestroy;
use crate::vulkan::r#impl::swapchain::SwapchainInfo;
use ultraviolet::{Rotor3, Vec3};
use vulkan_raw::{vkCmdSetScissor, vkCmdSetViewport, vkQueuePresentKHR, VkClearColorValue, VkClearDepthStencilValue, VkClearValue, VkCommandBufferLevel, VkCommandBufferUsageFlags, VkCommandPool, VkCommandPoolCreateFlags, VkDescriptorSet, VkExtent2D, VkExtent3D, VkFence, VkPipeline, VkPipelineBindPoint, VkPipelineLayout, VkPipelineStageFlags, VkPresentInfoKHR, VkQueue, VkRect2D, VkRenderPass, VkSampleCountFlags, VkSubpassContents, VkViewport};
use winit::keyboard::KeyCode;

const MAX_FRAMES_IN_FLIGHT: usize = 3;
#[derive(Default)]
pub struct RenderLoop {
    pub scene: Scene,
    pub settings: Settings,

    pub current_frame: usize,

    pub samples: VkSampleCountFlags,
    pub graph_pipeline_layout: VkDestroy<VkPipelineLayout>,
    pub graph_pipeline: VkDestroy<VkPipeline>,
    pub render_pass: VkDestroy<VkRenderPass>,
    pub descriptor_set: VkDescriptorSet,

    pub camera: Camera,

    pub graphic_queue: VkQueue,
    pub present_queue: VkQueue,
    pub extent: VkExtent2D,

    pub prepared: bool,

    pub command_pool: VkDestroy<VkCommandPool>,
    pub per_image_resources: Vec<PerImageResource>,
    pub per_frame_resources: Vec<PerFrameResource>,
}
pub const RAW: &[u8] = include_bytes!("../../hos.glb");

impl RenderLoop {
    pub fn recreate_framebuffers(&mut self, vulkan: &Vulkan, swapchain: &mut SwapchainInfo) {
        self.camera.set_aspect_ratio(swapchain.width as f32 / swapchain.height as f32);
        self.extent = VkExtent2D {
            width: swapchain.width,
            height: swapchain.height,
        };
        let extent = VkExtent3D {
            width: swapchain.width,
            height: swapchain.height,
            depth: 1,
        };

        let swapchain_images = vulkan.get_images(swapchain);
        self.per_image_resources = Vec::with_capacity(swapchain_images.len());

        swapchain_images.into_iter().for_each(|image| {
            self.per_image_resources.push(PerImageResource::new(vulkan, image, swapchain.format.format, extent, self.samples, *self.render_pass.get()));
        });
    }
    pub fn init(&mut self, vulkan: &Vulkan, swapchain: &mut SwapchainInfo, settings: &mut Settings) {
        self.scene = Scene::from_glb(RAW, vulkan.clone());

        let limits = &vulkan.get_loaded_device().device_info.properties.limits;
        let supported_samples = limits.framebufferColorSampleCounts & limits.framebufferDepthSampleCounts;
        self.samples = resolve_highest_multisampling(supported_samples, settings.msaa);
        let render_pass = vulkan.preset_renderpass_color_depth(self.samples, self.settings.render_format.format);
        self.render_pass = VkDestroy::new(render_pass, vulkan);

        self.recreate_framebuffers(vulkan, swapchain);

        let command_pool = vulkan.create_command_pool(vulkan.get_loaded_device().queue_info[0].family_index, VkCommandPoolCreateFlags::RESET_COMMAND_BUFFER_BIT);
        self.per_frame_resources = vulkan.alloc_command_buffers(command_pool, VkCommandBufferLevel::PRIMARY, MAX_FRAMES_IN_FLIGHT as u32).into_iter()
            .map(|command_buffer| {
                PerFrameResource::new(vulkan, command_buffer)
            }).collect::<Vec<_>>();
        self.command_pool = VkDestroy::new(command_pool, vulkan);

        let (preset, layout) = preset_graphic_pipeline(vulkan, swapchain.width, swapchain.height, render_pass, 0, &self.scene.descriptor_layouts);
        self.graph_pipeline_layout = VkDestroy::new(layout, vulkan);

        let create_info = preset_multisample(preset, supported_samples, settings.msaa);
        let graph_pipeline = create_pipelines_multithreaded(true, vec![create_info], vulkan)[0];
        self.graph_pipeline = VkDestroy::new(graph_pipeline, vulkan);

        //TODO: check for queues
        self.graphic_queue = vulkan.get_queues()[0];
        self.present_queue = vulkan.get_queues()[0];

        self.scene.ubo.set_view(self.camera.view_matrix());
        self.scene.ubo.set_proj(self.camera.projection_matrix());
        self.prepared = true;
    }

    pub fn render_loop(&mut self, vulkan: &Vulkan, swapchain: &mut SwapchainInfo, delta_time: f64) {
        if !self.prepared {
            return;
        }
        if self.extent.width != swapchain.width || self.extent.height != swapchain.height {
            self.extent.height = swapchain.height;
            vulkan.create_swapchain(swapchain);
            self.recreate_framebuffers(vulkan, swapchain);
            self.scene.ubo.set_proj(self.camera.projection_matrix());
        }

        let current_frame = self.current_frame;
        let frame_resource = &self.per_frame_resources[self.current_frame];

        vulkan.wait_for_fences(&[frame_resource.fence()], true, u64::MAX);
        vulkan.reset_fences(&[frame_resource.fence()]);

        let image_index = vulkan.get_next_image_index(swapchain, frame_resource.image_available_semaphore(), VkFence::none()) as usize;
        let image_resource = self.per_image_resources.get(image_index).unwrap();

        if self.camera.tick_speed(delta_time) {
            self.scene.ubo.set_view(self.camera.view_matrix());
        };

        let recording_info = RecordingInfo {
            renderPass: *self.render_pass.get(),
            subpass: 0,
            framebuffer: image_resource.framebuffer(),
            occlusionQueryEnable: false,
            queryFlags: Default::default(),
            pipelineStatistics: Default::default(),
        };
        vulkan.reset_buffer(frame_resource.command_buffer(), false);
        vulkan.start_recording(frame_resource.command_buffer(), VkCommandBufferUsageFlags::ONE_TIME_SUBMIT_BIT, recording_info);

        self.scene.ubo.sync_with_buffer(frame_resource.command_buffer(), vulkan);
        let clear_values = vec![
            VkClearValue { color: VkClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } },
            VkClearValue { depthStencil: VkClearDepthStencilValue { depth: 1.0, stencil: 0 } },
        ];
        vulkan.begin_render_pass(frame_resource.command_buffer(), *self.render_pass.get(), image_resource.framebuffer(),
                                  VkRect2D { offset: Default::default(), extent: self.extent }, clear_values.as_slice(), VkSubpassContents::INLINE);
        vulkan.bind_pipeline(frame_resource.command_buffer(), VkPipelineBindPoint::GRAPHICS, *self.graph_pipeline.get());

        let viewports = vec![VkViewport {
            x: 0.0,
            y: 0.0,
            width: swapchain.width as f32,
            height: swapchain.height as f32,
            minDepth: 0.1,
            maxDepth: 1.0,
        }];
        unsafe { vkCmdSetViewport(frame_resource.command_buffer(), 0, 1, viewports.as_ptr()); };

        let scissors = vec![VkRect2D {
            offset: Default::default(),
            extent: self.extent,
        }];
        unsafe { vkCmdSetScissor(frame_resource.command_buffer(), 0, 1, scissors.as_ptr()); };

        self.scene.render_scene(vulkan, frame_resource.command_buffer(), *self.graph_pipeline_layout.get());

        vulkan.end_render_pass(frame_resource.command_buffer());
        vulkan.end_recording(frame_resource.command_buffer());

        vulkan.submit_buffer(self.graphic_queue, frame_resource.fence(), &[frame_resource.command_buffer()],
                             &[WaitSemaphoreInfo {
                                 semaphore: frame_resource.image_available_semaphore(),
                                 waiting_stage: VkPipelineStageFlags::TOP_OF_PIPE_BIT,
                             }], &[image_resource.render_finished_semaphore()]);

        let present_info = VkPresentInfoKHR {
            waitSemaphoreCount: 1,
            pWaitSemaphores: &image_resource.render_finished_semaphore(),
            swapchainCount: 1,
            pSwapchains: &swapchain.swapchain,
            pImageIndices: &(image_index as u32),
            ..Default::default()
        };
        unsafe { vkQueuePresentKHR(self.present_queue, &present_info) };

        self.current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub fn handle_mouse_input(&mut self, delta: (f64, f64)) {
        let pitch = delta.1;
        let yaw = delta.0;

        let pitch_rotor = Rotor3::from_rotation_yz(pitch as f32);
        let yaw_rotor = Rotor3::from_rotation_xz(yaw as f32);

        self.camera.rotate(yaw_rotor, pitch_rotor);

        self.scene.ubo.set_view(self.camera.view_matrix());
    }

    pub fn key_pressed(&mut self, key: KeyCode) {
        let speed_vec = key_map(key);
        self.camera.add_speed(speed_vec);
    }

    pub fn key_released(&mut self, key: KeyCode) {
        let speed_vec = key_map(key);
        self.camera.remove_speed(speed_vec);
    }
}
//TODO: write keymap api with serialization
fn key_map(key_code: KeyCode) -> Vec3 {
    match key_code {
        KeyCode::KeyW => {
            -Vec3::unit_z()
        },
        KeyCode::KeyS => {
            Vec3::unit_z()
        },
        KeyCode::KeyD => {
            Vec3::unit_x()
        },
        KeyCode::KeyA => {
            -Vec3::unit_x()
        },
        KeyCode::Space => {
            Vec3::unit_y()
        },
        KeyCode::ShiftLeft => {
            -Vec3::unit_y()
        },
        _ => Vec3::zero(),
    }
}