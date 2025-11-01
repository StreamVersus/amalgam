use crate::vulkan::r#impl::device::LoadedDevice;
use std::cmp::min;
use vulkan_raw::{load_device_functions, load_instance_functions, ApiVersion, VkBool32, VkInstance, VkVersion};
#[derive(Default, Debug,  Clone)]
pub struct Vulkan {
    pub instance: Option<VkInstance>,
    pub loaded_device: Option<LoadedDevice>,
    api_version: ApiVersion,
}

impl Vulkan {
    pub fn init(&mut self) {
        self.create_instance();
        load_instance_functions(self.get_instance());
        self.create_logical_device();
        load_device_functions(self.get_loaded_device().logical_device);
        
        let instance_api_version = self.get_instance_api_version();
        let device_api_version = self.get_instance_api_version();
        self.api_version = min(instance_api_version, device_api_version);

        dbg!(self.api_version);
    }

    pub fn get_instance(&self) -> VkInstance {
        self.instance.expect("Tried to get instance, before initializing it")
    }
    
    pub fn get_loaded_device(&self) -> &LoadedDevice {
        self.loaded_device.as_ref().expect("Device not loaded")
    }

    pub fn is_version_supported(&self, api_version: VkVersion) -> bool {
        api_version as u32 >= self.api_version.into()
    }

    pub fn finish(&mut self) {
        self.destroy_logical_device();
        self.destroy_instance();
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
    if boolean == VkBool32::TRUE {
        true
    } else {
        false
    }
}
