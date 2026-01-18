use crate::engine::caches::cache::DeviceInfo;
use crate::vulkan::r#impl::device::LoadedDevice;

include!(concat!(env!("OUT_DIR"), "/cache.rs"));

pub fn build_device_info(device : &LoadedDevice) -> DeviceInfo {
    let properties = &device.device_info.properties;

    DeviceInfo {
        vendor_id: properties.vendorID,
        device_id: properties.deviceID,
        driver_version: properties.driverVersion,
    }
}
