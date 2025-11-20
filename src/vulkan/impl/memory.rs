use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::utils::align_up;
use std::os::raw::c_void;
use std::ptr::null_mut;
use vulkan_raw::{vkAllocateMemory, vkFlushMappedMemoryRanges, vkFreeMemory, vkMapMemory, vkUnmapMemory, VkBindBufferMemoryInfo, VkBindImageMemoryInfo, VkBuffer, VkDeviceMemory, VkImage, VkMappedMemoryRange, VkMemoryAllocateInfo, VkMemoryDedicatedAllocateInfo, VkMemoryMapFlagBits, VkMemoryPropertyFlags, VkMemoryRequirements, VkResult};


impl Vulkan {
    pub fn calculate_total_size(memory_requirements: &Vec<VkMemoryRequirements>) -> u64 {
        let mut total_size = 0u64;

        for req in memory_requirements {
            total_size = align_up(total_size, req.alignment);
            total_size += req.size;
        }

        total_size
    }

    fn allocate_memory_object(&self, total_size: u64, memory_type: u32) -> VkDeviceMemory {
        let allocate_info = VkMemoryAllocateInfo {
            allocationSize: total_size,
            memoryTypeIndex: memory_type,
            ..Default::default()
        };
        let mut memory_object = VkDeviceMemory::none();
        let result = unsafe { vkAllocateMemory(self.get_loaded_device().logical_device, &allocate_info, null_mut(), &mut memory_object) };
        assert_eq!(result, VkResult::SUCCESS);

        memory_object
    }

    #[allow(private_bounds)]
    fn allocate_dedicated_memory(&self, resource: &dyn Any, req: VkMemoryRequirements, flags: VkMemoryPropertyFlags) -> VkDeviceMemory {
        let mut image = VkImage::none();
        let mut buffer = VkBuffer::none();
        if let Some(&i) = resource.downcast_ref::<VkImage>() {
            image = i;
        } else if let Some(&i) = resource.downcast_ref::<VkBuffer>() {
            buffer = i;
        } else {
            panic!("Unexpected resource type");
        }

        let dedicated_info = VkMemoryDedicatedAllocateInfo {
            image,
            buffer,
            ..Default::default()
        };
        let allocate_info = VkMemoryAllocateInfo {
            pNext: &dedicated_info as *const _ as *const c_void,
            allocationSize: req.size,
            memoryTypeIndex: self.find_memory_type(&[req], flags).unwrap(),
            ..Default::default()
        };

        let mut memory_object = VkDeviceMemory::none();
        let result = unsafe { vkAllocateMemory(self.get_loaded_device().logical_device, &allocate_info, null_mut(), &mut memory_object) };
        assert_eq!(result, VkResult::SUCCESS);

        memory_object
    }

    pub fn map_memory(&self, info: &MemoryInfo) -> *mut c_void {
        let mut pointer: *mut c_void = null_mut();
        let result = unsafe{ vkMapMemory(self.get_loaded_device().logical_device, info.memory_object, info.offset, info.data_size, VkMemoryMapFlagBits::empty(), &mut pointer) };
        assert_eq!(result, VkResult::SUCCESS);
        
        pointer
    }

    pub fn flush_memory(&self, info: &[MemoryInfo]) -> bool {
        let memory_ranges = info.iter().map(|info| VkMappedMemoryRange {
            memory: info.memory_object,
            offset: info.offset,
            size: info.data_size,
            ..Default::default()
        }).collect::<Vec<_>>();

        let result = unsafe { vkFlushMappedMemoryRanges(self.get_loaded_device().logical_device, memory_ranges.len() as u32, memory_ranges.as_ptr()) };
        result == VkResult::SUCCESS
    }

    /// use with caution, double check for alignment
    #[allow(deprecated)]
    pub fn copy_info<T>(dst_pointer: *mut c_void, src_pointer: *const T, count: usize) {
        unsafe { std::intrinsics::copy_nonoverlapping(src_pointer as *const u8, dst_pointer as *mut u8, size_of::<T>() * count) };
    }

    pub fn destroy_memory(&self, memory: VkDeviceMemory) {
        unsafe { vkFreeMemory(self.get_loaded_device().logical_device, memory, null_mut()) };
    }

    pub fn unmap_memory(&self, memory: &MemoryInfo) {
        unsafe { vkUnmapMemory(self.get_loaded_device().logical_device, memory.memory_object) };
    }

    pub fn find_memory_type(&self, memory_requirements: &[VkMemoryRequirements], properties: VkMemoryPropertyFlags) -> Option<u32> {
        let memory_type_bits = memory_requirements
            .iter()
            .map(|req| req.memoryTypeBits)
            .fold(u32::MAX, |acc, bits| acc & bits);

        let mem_properties = &self.get_loaded_device().memory_properties;

        for i in 0..mem_properties.memoryTypeCount {
            let type_supported = (memory_type_bits & (1 << i)) != 0;

            let has_properties = (mem_properties.memoryTypes[i as usize].propertyFlags & properties) == properties;

            if type_supported && has_properties {
                return Some(i);
            }
        }

        None
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct MemoryInfo {
    pub memory_object: VkDeviceMemory,
    pub offset: u64,
    pub data_size: u64,
}

impl MemoryInfo {
    pub fn memory_object(&self) -> VkDeviceMemory {
        self.memory_object
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn data_size(&self) -> u64 {
        self.data_size
    }
}

impl Destructible for VkDeviceMemory {
    fn destroy(&self, vulkan: &Vulkan) {
        vulkan.destroy_memory(*self);
    }
}

pub struct AllocationTask {
    allocatables: Vec<Box<dyn Allocatable>>,
    flags: VkMemoryPropertyFlags,
}

impl AllocationTask {
    pub fn new(flags: VkMemoryPropertyFlags) -> Self {
        AllocationTask {
            allocatables: vec![],
            flags,
        }
    }
    
    pub fn device() -> Self {
        Self::new(VkMemoryPropertyFlags::DEVICE_LOCAL_BIT)
    }
    
    pub fn host_cached() -> Self {
        Self::new(VkMemoryPropertyFlags::HOST_VISIBLE_BIT | VkMemoryPropertyFlags::HOST_CACHED_BIT)
    }

    pub fn host_coherent() -> Self {
        Self::new(VkMemoryPropertyFlags::HOST_VISIBLE_BIT | VkMemoryPropertyFlags::HOST_COHERENT_BIT)
    }

    #[allow(private_bounds)]
    pub fn add_allocatable<T: Allocatable + 'static>(mut self, dest: T) -> Self{
        self.allocatables.push(Box::new(dest));
        self
    }

    #[allow(private_bounds)]
    pub fn add_allocatables<T: Allocatable + 'static>(mut self, dest: Vec<T>) -> Self{
        self.allocatables.reserve(dest.len());
        self.allocatables.extend(dest.into_iter().map(|item| Box::new(item) as Box<dyn Allocatable>));
        self
    }

    #[allow(private_bounds)]
    pub fn add_allocatable_ref<T: Allocatable + 'static>(&mut self, dest: T) {
        self.allocatables.push(Box::new(dest));
    }

    #[allow(private_bounds)]
    pub fn add_allocatables_ref<T: Allocatable + 'static>(&mut self, dest: Vec<T>) {
        self.allocatables.reserve(dest.len());
        self.allocatables.extend(dest.into_iter().map(|item| Box::new(item) as Box<dyn Allocatable>));
    }

    fn add_bind_task(buffer_bind_tasks: &mut Vec<VkBindBufferMemoryInfo>, image_bind_tasks: &mut Vec<VkBindImageMemoryInfo>,info: &mut AllocationInfo, any: &dyn Any, memory: VkDeviceMemory, offset: u64, size: u64) {
        if let Some(&buffer) = any.downcast_ref::<VkBuffer>() {
            let task = VkBindBufferMemoryInfo {
                buffer,
                memory,
                memoryOffset: offset,
                ..Default::default()
            };
            buffer_bind_tasks.push(task);
            info.store_buffer_info(size, task);
        } else if let Some(&image) = any.downcast_ref::<VkImage>() {
            let task = VkBindImageMemoryInfo {
                image,
                memory,
                memoryOffset: offset,
                ..Default::default()
            };
            image_bind_tasks.push(task);
            info.store_image_info(size, task);
        }
    }

    pub fn allocate_all(self, vulkan: &Vulkan) -> AllocationInfo {
        let mut info = AllocationInfo::default();
        if self.allocatables.is_empty() {
            return info;
        }

        let mut requirements: Vec<VkMemoryRequirements> = Vec::new();
        let mut total_size = 0u64;

        let mut buffer_bind_tasks: Vec<VkBindBufferMemoryInfo> = Vec::new();
        let mut image_bind_tasks: Vec<VkBindImageMemoryInfo> = Vec::new();

        let atom_size = vulkan.get_loaded_device().device_info.properties.limits.nonCoherentAtomSize;
        for allocatable in &self.allocatables {
            let (req, is_dedicated) = allocatable.get_memory_requirements(vulkan);
            if is_dedicated {
                let aligned_size = align_up(req.size, atom_size);
                let mut dedicated_req = req.clone();
                dedicated_req.size = aligned_size;

                let memory = vulkan.allocate_dedicated_memory(allocatable.as_any(), req.clone(), self.flags);
                Self::add_bind_task(&mut buffer_bind_tasks, &mut image_bind_tasks, &mut info, allocatable.as_any(), memory, 0, req.size);
            } else {
                total_size = align_up(total_size, req.alignment);
                total_size += align_up(req.size, atom_size);

                requirements.push(req);
            }
        }
        total_size = align_up(total_size, atom_size);

        let memory_type_index = vulkan.find_memory_type(
            &requirements,
            self.flags,
        ).expect("Failed to find suitable memory type");

        let memory = vulkan.allocate_memory_object(total_size, memory_type_index);

        let mut current_offset = 0u64;
        let allocatable_iter = self.allocatables.into_iter();
        let requirements_iter = requirements.into_iter();

        for (allocatable, req) in allocatable_iter.zip(requirements_iter) {
            current_offset = align_up(current_offset, req.alignment.max(atom_size));
            Self::add_bind_task(&mut buffer_bind_tasks, &mut image_bind_tasks, &mut info, allocatable.as_any(), memory, current_offset, align_up(req.size, atom_size));
            current_offset += align_up(req.size, atom_size);
        }

        vulkan.bind_memory_to_buffer(buffer_bind_tasks);
        vulkan.bind_memory_to_image(image_bind_tasks);

        info
    }
}

#[derive(Default)]
pub struct AllocationInfo {
    buffer_info: HashMap<VkBuffer, MemoryInfo>,
    image_info: HashMap<VkImage, MemoryInfo>,
}

impl AllocationInfo {

    pub fn store_buffer_info(&mut self, size: u64, info: VkBindBufferMemoryInfo) {
        self.buffer_info.insert(info.buffer, MemoryInfo {
            memory_object: info.memory,
            offset: info.memoryOffset,
            data_size: size,
        });
    }

    pub fn store_image_info(&mut self, size: u64, info: VkBindImageMemoryInfo) {
        self.image_info.insert(info.image, MemoryInfo {
            memory_object: info.memory,
            offset: info.memoryOffset,
            data_size: size,
        });
    }

    pub fn merge_mut(&mut self, allocation_info: AllocationInfo) {
        self.buffer_info.extend(allocation_info.buffer_info);
        self.image_info.extend(allocation_info.image_info);
    }

    pub fn merge(allocation_info: AllocationInfo)  -> Self {
        let mut info = AllocationInfo::default();
        info.merge_mut(allocation_info);
        info
    }

    pub fn merge_all(allocation_infos: Vec<AllocationInfo>)  -> Self {
        let mut info = AllocationInfo::default();
        allocation_infos.into_iter().for_each(|all_info| info.merge_mut(all_info));
        info
    }

    pub fn pull_buffer_info(&self, buffer: &VkBuffer) -> MemoryInfo {
        *self.buffer_info.get(buffer).unwrap()
    }

    pub fn pull_image_info(&self, image: &VkImage) -> MemoryInfo {
        *self.image_info.get(image).unwrap()
    }

    pub fn get_all_memory_objects(&self) -> Vec<VkDeviceMemory> {
        let mut memory_objects = Vec::with_capacity(self.buffer_info.len() + self.image_info.len());

        self.buffer_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object);
        });
        self.image_info.iter().for_each(|(_, info)| {
            memory_objects.push(info.memory_object);
        });

        memory_objects
    }
}
trait Allocatable: Send + Sync + Any {
    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool);

    fn as_any(&self) -> &dyn Any;
}

impl Hash for dyn Allocatable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(buffer) = self.as_any().downcast_ref::<VkBuffer>() {
            buffer.hash(state);
        } else if let Some(image) = self.as_any().downcast_ref::<VkImage>() {
            image.hash(state);
        }
    }
}

impl PartialEq for dyn Allocatable {
    fn eq(&self, other: &Self) -> bool {
        self == other
    }
}

impl Eq for dyn Allocatable {}

pub trait AllocatableCollection {
    fn allocate_all(self, vulkan: &mut Vulkan, flags: VkMemoryPropertyFlags);
}

impl Allocatable for VkBuffer {

    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool) {
        vulkan.get_buffer_memory_requirements(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AllocatableCollection for Vec<VkBuffer> {
    fn allocate_all(self, vulkan: &mut Vulkan, flags: VkMemoryPropertyFlags) {
        AllocationTask::new(flags).add_allocatables(self).allocate_all(vulkan);
    }
}

impl Allocatable for VkImage {

    fn get_memory_requirements(&self, vulkan: &Vulkan) -> (VkMemoryRequirements, bool) {
        vulkan.get_image_memory_requirements(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AllocatableCollection for Vec<VkImage> {
    fn allocate_all(self, vulkan: &mut Vulkan, flags: VkMemoryPropertyFlags) {
        AllocationTask::new(flags).add_allocatables(self).allocate_all(vulkan);
    }
}

#[derive(Default, Debug)]
pub struct VkDestroy<T: Destructible + Default + PartialEq> {
    object: T,
    vulkan: Vulkan,
}

impl<T: Destructible + Default + PartialEq> VkDestroy<T> {
    pub fn new(object: T, vulkan: &Vulkan) -> Self {
        Self { object, vulkan: vulkan.clone() }
    }

    pub fn get(&self) -> &T {
        &self.object
    }
}

impl<T: Destructible + Default + PartialEq> Drop for VkDestroy<T> {
    fn drop(&mut self) {
        if self.object != T::default() {
            self.object.destroy(&self.vulkan);
        }
    }
}

impl<T: Destructible + Default + PartialEq> Deref for VkDestroy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}