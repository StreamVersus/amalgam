use crate::prelude::LoadedDevice;
use std::io::Error;
use vulkan_raw::{load_device_functions, load_instance_functions, ApiVersion, VkBool32, VkInstance, VkVersion};
use crate::application::{ENGINE_MAJOR_VERSION, ENGINE_MINOR_VERSION, ENGINE_PATCH_VERSION};
use crate::prelude::arena_alloc::ArenaAllocator;
use crate::prelude::pool_alloc::PoolAllocator;

#[derive(Default, Debug, Clone)]
pub struct Vulkan {
    pub instance: Option<VkInstance>,
    pub loaded_device: Option<LoadedDevice>,
    api_version: ApiVersion,
    vma: PoolAllocator,
}

impl Vulkan {
    pub fn init(&mut self) {
        self.api_version = ApiVersion::new(ENGINE_MAJOR_VERSION, ENGINE_MINOR_VERSION, ENGINE_PATCH_VERSION);

        self.create_instance();
        load_instance_functions(self.get_instance());
        self.create_logical_device();
        load_device_functions(self.get_loaded_device().logical_device);

        self.vma = PoolAllocator::new(self);
        dbg!(self.api_version);
    }

    pub fn get_instance(&self) -> VkInstance {
        self.instance.expect("Tried to get instance, before initializing it")
    }
    
    pub fn get_loaded_device(&self) -> &LoadedDevice {
        self.loaded_device.as_ref().expect("Device not loaded")
    }

    pub fn safe_get_loaded_device(&self) -> Result<&LoadedDevice, Error> {
        self.loaded_device.as_ref().ok_or(Error::other("Device not loaded".to_string()))
    }

    pub fn is_version_supported(&self, api_version: VkVersion) -> bool {
        api_version as u32 >= self.api_version.into()
    }

    pub fn get_api_version(&self) -> ApiVersion {
        self.api_version
    }

    pub fn finish(&mut self) {
        self.destroy_logical_device();
        self.destroy_instance();
    }

    pub fn arena(&self) -> ArenaAllocator {
        ArenaAllocator::default()
    }

    pub fn pool(&self) -> &PoolAllocator {
        &self.vma
    }
}

pub trait Destructible: Send + Sync {
    fn destroy(&self, vulkan: &Vulkan);
}

unsafe impl Send for Vulkan {}
unsafe impl Sync for Vulkan {}

#[inline(always)]
pub fn bool_to_vkbool(boolean: bool) -> VkBool32 {
    if boolean {
        VkBool32::TRUE
    } else {
        VkBool32::FALSE
    }
}

#[inline(always)]
pub fn vkbool_to_bool(boolean: VkBool32 ) -> bool {
    boolean == VkBool32::TRUE
}
