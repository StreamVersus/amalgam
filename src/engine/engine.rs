use crate::both::RenderLoop;
use crate::engine::{App, Delta};
use crate::vulkan::func::Vulkan;
use crate::vulkan::r#impl::memory::{AllocationTask, VkDestroy};
use crate::vulkan::r#impl::surface::SurfaceFormat;
use crate::vulkan::r#impl::swapchain::SwapchainInfo;
use crate::vulkan::utils::ImageUsage;
use vulkan_raw::{VkCommandBuffer, VkDeviceMemory, VkExtent3D, VkFence, VkFormat, VkFramebuffer, VkImage, VkImageAspectFlags, VkImageType, VkImageView, VkImageViewType, VkRenderPass, VkSampleCountFlags, VkSemaphore};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::platform::x11::EventLoopBuilderExtX11;

pub fn create_window(settings: Settings) {
    let mut vulkan = Vulkan::default();
    vulkan.init();
    let mut event_loop = EventLoop::builder();

    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        event_loop.with_android_app((&settings).activity.clone().unwrap());
    }

    #[cfg(target_os = "linux")]
    {
        if crate::vulkan::platform::is_wayland() {
            event_loop.with_wayland();
        } else {
            event_loop.with_x11();
        }
    }
    
    let event_loop = event_loop.build().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        window: None,
        delta: Delta::new(settings.target_fps, settings.min_fps, settings.smoothing_factor, settings.vsync),
        settings,
        vulkan,
        swapchain_info: Default::default(),
        render_loop: Default::default(),
        pressed_keys: Default::default(),
        focused: false,
    };
    event_loop.run_app(&mut app).unwrap()
}

//TODO: populate with another options
pub struct Settings {
    pub width: u32,
    pub height: u32,
    pub render_format: SurfaceFormat,
    pub target_fps: f64,
    pub min_fps: f64,
    pub smoothing_factor: f64,
    pub vsync: bool,
    pub sensitivity: (f64, f64),
    pub msaa: VkSampleCountFlags,
    pub callbacks: Callbacks,
    #[cfg(target_os = "android")]
    pub activity: Option<android_activity::AndroidApp>,
}
pub struct Callbacks {
    pub render_init: fn(&mut RenderLoop, &Vulkan, &mut SwapchainInfo, &mut Settings),
    pub render: fn(&mut RenderLoop, &Vulkan, &mut SwapchainInfo, f64),
    pub handle_mouse: fn(&mut RenderLoop, (f64, f64)),
    pub key_pressed: fn(&mut RenderLoop, KeyCode),
    pub key_released: fn(&mut RenderLoop, KeyCode),
}

impl Default for Callbacks {
    fn default() -> Self {
        Callbacks {
            render_init: |_a, _b, _c, _d| {}, // no-op implementation
            render: |_a, _b, _c, _d| {},
            handle_mouse: |_a, _b| {},
            key_pressed: |_a, _b| {},
            key_released: |_a, _b| {},
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            width: 640,
            height: 480,
            render_format: Default::default(),
            target_fps: 144.0,
            min_fps: 24.0,
            smoothing_factor: 0.7,
            vsync: false,
            sensitivity: (1.0, 1.0),
            msaa: VkSampleCountFlags::SC_1_BIT,
            callbacks: Default::default(),
            #[cfg(target_os = "android")]
            activity: None,
        }
    }
}

pub struct PerFrameResource {
    fence: VkDestroy<VkFence>,
    image_available_semaphore: VkDestroy<VkSemaphore>,
    command_buffer: VkCommandBuffer,
}

impl PerFrameResource {
    pub fn new(vulkan: &Vulkan, command_buffer: VkCommandBuffer) -> Self {
        let fence = vulkan.create_fence(true);
        let image_available_semaphore = vulkan.create_semaphore();

        Self {
            fence: VkDestroy::new(fence, vulkan),
            image_available_semaphore: VkDestroy::new(image_available_semaphore, vulkan),
            command_buffer,
        }
    }

    pub fn fence(&self) -> VkFence {
        *self.fence
    }

    pub fn image_available_semaphore(&self) -> VkSemaphore {
        *self.image_available_semaphore
    }

    pub fn command_buffer(&self) -> VkCommandBuffer {
        self.command_buffer
    }
}

pub struct PerImageResource {
    swapchain_image: VkImage,
    swapchain_image_view: VkDestroy<VkImageView>,
    depth_image: VkDestroy<VkImage>,
    depth_image_view: VkDestroy<VkImageView>,
    color_msaa_image: Option<VkDestroy<VkImage>>,
    color_msaa_view: Option<VkDestroy<VkImageView>>,
    framebuffer: VkDestroy<VkFramebuffer>,
    render_finished_semaphore: VkDestroy<VkSemaphore>,
    _memory_storage: Vec<VkDestroy<VkDeviceMemory>>,
}

impl PerImageResource {
    pub fn new(vulkan: &Vulkan, image: VkImage, format: VkFormat, extent: VkExtent3D, sample_rate: VkSampleCountFlags, render_pass: VkRenderPass) -> Self {
        let swapchain_image_view = vulkan.create_image_view(&image, VkImageViewType::IVT_2D, format, VkImageAspectFlags::COLOR_BIT);
        let depth_image = vulkan.create_image(VkFormat::D32_SFLOAT, VkImageType::IT_2D, false, 1, 1, extent, sample_rate, ImageUsage::default().depth_stencil_attachment(true));
        let mut device_task = AllocationTask::device();
        device_task.add_allocatable_ref(depth_image);

        let color_msaa_image = if sample_rate != VkSampleCountFlags::SC_1_BIT {
            let color_msaa_image = vulkan.create_image(format, VkImageType::IT_2D, false, 1, 1, extent, sample_rate, ImageUsage::default().color_attachment(true));
            device_task.add_allocatable_ref(color_msaa_image);

            Some(VkDestroy::new(color_msaa_image, vulkan))
        } else {
            None
        };
        let memory = device_task.allocate_all(vulkan).get_all_memory_objects();
        let memory_storage = memory.into_iter().map(|memory| {
            VkDestroy::new(memory, vulkan)
        }).collect();

        let color_msaa_view = if sample_rate != VkSampleCountFlags::SC_1_BIT {
            let color_msaa_view = vulkan.create_image_view(color_msaa_image.as_ref().unwrap().get(), VkImageViewType::IVT_2D, format, VkImageAspectFlags::COLOR_BIT);
            Some(VkDestroy::new(color_msaa_view, vulkan))
        } else {
            None
        };

        let depth_image_view = vulkan.create_image_view(&depth_image, VkImageViewType::IVT_2D, VkFormat::D32_SFLOAT, VkImageAspectFlags::DEPTH_BIT);
        let attachments = if sample_rate != VkSampleCountFlags::SC_1_BIT {
            vec![*color_msaa_view.as_ref().unwrap().get(), depth_image_view, swapchain_image_view]
        } else {
            vec![swapchain_image_view, depth_image_view]
        };

        let framebuffer = vulkan.create_framebuffer(render_pass, &attachments, extent.width, extent.height, 1);

        let render_finished_semaphore = vulkan.create_semaphore();
        Self {
            swapchain_image: image,
            swapchain_image_view: VkDestroy::new(swapchain_image_view, vulkan),
            depth_image: VkDestroy::new(depth_image, vulkan),
            depth_image_view: VkDestroy::new(depth_image_view, vulkan),
            color_msaa_image,
            color_msaa_view,
            framebuffer: VkDestroy::new(framebuffer, vulkan),
            render_finished_semaphore: VkDestroy::new(render_finished_semaphore, vulkan),
            _memory_storage: memory_storage,
        }
    }

    pub fn swapchain_image(&self) -> VkImage {
        self.swapchain_image
    }

    pub fn swapchain_image_view(&self) -> VkImageView {
        *self.swapchain_image_view
    }

    pub fn depth_image(&self) -> VkImage {
        *self.depth_image
    }

    pub fn depth_image_view(&self) -> VkImageView {
        *self.depth_image_view
    }

    pub fn color_msaa_image(&self) -> &Option<VkDestroy<VkImage>> {
        &self.color_msaa_image
    }

    pub fn color_msaa_view(&self) -> &Option<VkDestroy<VkImageView>> {
        &self.color_msaa_view
    }

    pub fn framebuffer(&self) -> VkFramebuffer {
        *self.framebuffer
    }

    pub fn render_finished_semaphore(&self) -> VkSemaphore {
        *self.render_finished_semaphore
    }
}