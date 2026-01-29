use crate::safe_ptr;
use crate::vulkan::func::Vulkan;
use crate::vulkan::platform::device_extensions;
use crate::vulkan::r#impl::queues::QueueInfo;
use crate::vulkan::utils::{null_terminated_str, null_terminated_string};
use std::collections::HashSet;
use std::ffi::{c_char, c_void};
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};
use vulkan_raw::{vkCreateDevice, vkDestroyDevice, vkEnumerateDeviceExtensionProperties, vkEnumeratePhysicalDevices, vkGetPhysicalDeviceFeatures, vkGetPhysicalDeviceFormatProperties, vkGetPhysicalDeviceMemoryProperties, vkGetPhysicalDeviceProperties, ApiVersion, VkBool32, VkDevice, VkDeviceCreateInfo, VkDeviceQueueCreateInfo, VkExtensionProperties, VkFormat, VkFormatProperties, VkPhysicalDevice, VkPhysicalDeviceFeatures, VkPhysicalDeviceMemoryProperties, VkPhysicalDeviceProperties, VkPhysicalDeviceVulkan12Features, VkQueue};

impl Vulkan {
    pub fn get_devices(&self) -> Vec<VkPhysicalDevice> {
        let mut device_count: u32 = 0;
        let instance = self.get_instance();

        let mut result = unsafe {
             vkEnumeratePhysicalDevices(instance, &mut device_count, null_mut())
        };
        
        assert_ne!(device_count, 0);
        assert!(result.is_ok());

        let mut devices: Vec<VkPhysicalDevice> = Vec::with_capacity(device_count as usize);
        let spare = devices.spare_capacity_mut();
        result = unsafe {
            vkEnumeratePhysicalDevices(instance, &mut device_count, spare.as_mut_ptr() as *mut VkPhysicalDevice)
        };
        assert_ne!(device_count, 0);
        assert!(result.is_ok());

        unsafe {
            devices.set_len(device_count as usize);
        }
        devices
    }
    
    pub fn get_physical_device_extensions(&self, device: VkPhysicalDevice) -> Vec<VkExtensionProperties> {
        let mut extension_count: u32 = 0;

        let mut result = unsafe {
            vkEnumerateDeviceExtensionProperties(device, null_mut(), &mut extension_count, null_mut())
        };
        assert_ne!(extension_count, 0);
        assert!(result.is_ok());

        let mut extensions : Vec<VkExtensionProperties> = Vec::with_capacity(extension_count as usize);
        let spare = extensions.spare_capacity_mut();
        result = unsafe {
            vkEnumerateDeviceExtensionProperties(device, null_mut(), &mut extension_count, spare.as_mut_ptr() as *mut VkExtensionProperties)
        };
        assert_ne!(extension_count, 0);
        assert!(result.is_ok());

        unsafe {
            extensions.set_len(extension_count as usize);
        }
        extensions
    }
    
    pub fn get_physical_device_info(&self, device: VkPhysicalDevice) -> DeviceInfo {
        let mut features: MaybeUninit<VkPhysicalDeviceFeatures> = MaybeUninit::uninit();
        let mut properties: MaybeUninit<VkPhysicalDeviceProperties> = MaybeUninit::uninit();

        unsafe {
            vkGetPhysicalDeviceFeatures(device, features.as_mut_ptr());
            vkGetPhysicalDeviceProperties(device, properties.as_mut_ptr());
        }

        let features: VkPhysicalDeviceFeatures = unsafe { features.assume_init() };
        let properties: VkPhysicalDeviceProperties = unsafe { properties.assume_init() };
        
        DeviceInfo {
            features,
            properties,
        }
    }

    pub fn create_logical_device(&mut self) {
        let devices = self.get_devices();
        let desired_extensions: HashSet<String> = device_extensions()
            .into_iter()
            .map(null_terminated_str)
            .collect();
        for device in devices {
            let extension_data = self.get_physical_device_extensions(device);
            let extensions: HashSet<String> = extension_data
                .iter()
                .map(|ext| null_terminated_string(&ext.extensionName))
                .collect();
            if !desired_extensions.iter().all(|ext| extensions.contains(ext)) {
                continue;
            }

            let mut device_info = self.get_physical_device_info(device);
            if device_info.features.geometryShader != VkBool32::TRUE || device_info.features.multiDrawIndirect != VkBool32::TRUE {
                continue;
            }
            else {
                device_info.features = VkPhysicalDeviceFeatures::default();

                device_info.features.geometryShader = VkBool32::TRUE;
                device_info.features.multiDrawIndirect = VkBool32::TRUE;
            };
            let queue_info = self.build_desired_queue_info(device);
            let queue_create_info: Vec<VkDeviceQueueCreateInfo> = queue_info
                .iter()
                .map(|x| VkDeviceQueueCreateInfo {
                    queueFamilyIndex: x.family_index,
                    queueCount: x.priorities.len() as u32,
                    pQueuePriorities: x.priorities.as_ptr(),
                    ..Default::default()
                })
                .collect();

            let device_extension_ptrs: Vec<*const c_char> = desired_extensions
                .iter()
                .map(|extension| extension.as_ptr() as *const c_char)
                .collect();

            #[allow(unused_mut)]
            let mut layers: Vec<*const c_char> = vec![];
            layers.push(c"VK_AMD_anti_lag".as_ptr() as *const c_char);

            let vulkan_12_features = VkPhysicalDeviceVulkan12Features {
                pNext: null_mut(),
                vulkanMemoryModel: VkBool32::TRUE,
                ..Default::default()
            };

            let device_create_info = VkDeviceCreateInfo {
                pNext: &vulkan_12_features as *const _ as *const c_void,
                queueCreateInfoCount: queue_create_info.len() as u32,
                pQueueCreateInfos: queue_create_info.as_ptr(),
                enabledLayerCount: layers.len() as u32,
                ppEnabledLayerNames: safe_ptr!(layers),
                enabledExtensionCount: desired_extensions.len() as u32,
                ppEnabledExtensionNames: safe_ptr!(device_extension_ptrs),
                pEnabledFeatures: &device_info.features,
                ..Default::default()
            };
            let mut logical_device = VkDevice::none();
            let result = unsafe { vkCreateDevice(device, &device_create_info, null(), &mut logical_device) };
            assert!(result.is_ok());
            
            let memory_properties = self.get_memory_properties(device);

            let device_info = self.get_physical_device_info(device);
            
            let loaded_device = LoadedDevice {
                logical_device,
                device,
                queue_info,
                graphic_queue: Default::default(),
                memory_properties,
                device_info
            };
            
            self.loaded_device = Some(loaded_device);
            return;
        }
        panic!("Unable to find suitable device");
    }

    pub fn get_memory_properties(&self, device: VkPhysicalDevice) -> VkPhysicalDeviceMemoryProperties {
        let mut ph: MaybeUninit<VkPhysicalDeviceMemoryProperties> = MaybeUninit::uninit();
        unsafe {
            vkGetPhysicalDeviceMemoryProperties(device, ph.as_mut_ptr());
        };

        unsafe { ph.assume_init() }
    }
    
    pub fn get_format_properties(&self, format: VkFormat) -> VkFormatProperties {
        let mut ph: MaybeUninit<VkFormatProperties> = MaybeUninit::uninit();
        unsafe { vkGetPhysicalDeviceFormatProperties(self.get_loaded_device().device, format, ph.as_mut_ptr()) };

        unsafe{ ph.assume_init() }
    }

    pub fn get_device_vulkan_version(&self) -> ApiVersion {
        ApiVersion::from(self.get_loaded_device().device_info.properties.apiVersion)
    }
    
    pub fn destroy_logical_device(&mut self) {
        unsafe { vkDestroyDevice(self.get_loaded_device().logical_device, null_mut()) };
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub features: VkPhysicalDeviceFeatures,
    pub properties: VkPhysicalDeviceProperties,
}
#[derive(Debug, Clone)]
pub struct LoadedDevice {
    pub logical_device: VkDevice,
    pub device: VkPhysicalDevice,
    pub queue_info: Vec<QueueInfo>,
    pub graphic_queue: VkQueue,
    pub memory_properties: VkPhysicalDeviceMemoryProperties,
    pub device_info: DeviceInfo,
}
