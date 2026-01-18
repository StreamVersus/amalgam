use crate::application::{APPLICATION_NAME, ENGINE_MAJOR_VERSION, ENGINE_MINOR_VERSION, ENGINE_NAME, ENGINE_PATCH_VERSION};
use crate::safe_ptr;
use crate::vulkan::func::Vulkan;
use crate::vulkan::platform::{extensions, platform_extensions};
use crate::vulkan::utils::{null_terminated_str, null_terminated_string};
use std::collections::HashSet;
use std::ffi::c_char;
use std::ptr::null_mut;
use vulkan_raw::{vkCreateInstance, vkDestroyInstance, vkEnumerateInstanceVersion, ApiVersion, VkApplicationInfo, VkInstance, VkInstanceCreateInfo};

impl Vulkan {
    pub fn create_instance(&mut self) {
        let supported_vec = Vulkan::get_extensions();
        let supported: HashSet<String> = supported_vec
            .iter()
            .map(|x| null_terminated_string(&x.extensionName))
            .collect();
        let desired_extensions: HashSet<String> = [extensions().as_slice(), platform_extensions(&supported).as_slice()]
            .concat()
            .into_iter()
            .map(|str| null_terminated_str(str))
            .collect();

        if !desired_extensions.iter().all(|x| supported.contains(x)) {
            panic!("The Vulkan API does not support specified extensions");
        };

        let app_name = null_terminated_str(APPLICATION_NAME);
        let engine_name = null_terminated_str(ENGINE_NAME);
        let application_info = VkApplicationInfo {
            pApplicationName: app_name.as_ptr() as *const c_char,
            applicationVersion: 1,
            pEngineName: engine_name.as_ptr() as *const c_char,
            engineVersion: 1,
            apiVersion: ApiVersion::new(ENGINE_MAJOR_VERSION, ENGINE_MINOR_VERSION, ENGINE_PATCH_VERSION).into(),
            ..Default::default()
        };

        let mut extension_ptrs: Vec<*const c_char> = Vec::with_capacity(desired_extensions.len());
        extension_ptrs.extend(desired_extensions.iter().map(|ext| ext.as_ptr() as *const c_char));

        #[allow(unused_mut)]
        let mut layers: Vec<*const c_char> = vec![];
        #[cfg(target_os="linux")]
        {
            #[cfg(debug_assertions)] layers.push("VK_LAYER_KHRONOS_validation\0".as_ptr() as *const c_char);
        }
        let instance_info = VkInstanceCreateInfo {
            pApplicationInfo: &application_info,
            enabledLayerCount: layers.len() as u32,
            ppEnabledLayerNames: layers.as_ptr(),
            enabledExtensionCount: desired_extensions.len() as u32,
            ppEnabledExtensionNames: safe_ptr!(extension_ptrs),
            ..Default::default()
        };

        let mut instance = VkInstance::none();
        let result = unsafe {
            vkCreateInstance(&instance_info, null_mut(), &mut instance)
        };
        assert!(result.is_ok());
        
        self.instance = Some(instance);
    }
    
    pub fn get_instance_api_version(&self) -> ApiVersion {
        let mut api_version = 0u32;
        unsafe { vkEnumerateInstanceVersion(&mut api_version) };
        
        ApiVersion::from(api_version)
    }
    
    pub fn destroy_instance(&mut self) {
        let instance = self.instance.take();
        if instance.is_some() {
            let instance = instance.unwrap();
            unsafe { vkDestroyInstance(instance, null_mut()); }
        }
    }
}