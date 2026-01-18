use crate::engine::caches::{build_device_info, Cache};
use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::r#impl::pipelines::GraphicsPipelineCreateInfo;
use prost::Message;
use std::cmp::max;
use std::fs;
use std::fs::File;
use std::sync::Arc;
use std::thread::available_parallelism;
use vulkan_raw::{VkPipeline, VkPipelineCache};

const FILE_PATH: &str = "cache.storage";
const VERSION: u32 = 0;

pub fn create_pipelines_multithreaded(use_caches: bool, pipeline_infos: Vec<GraphicsPipelineCreateInfo>, vulkan: &Vulkan) -> Vec<VkPipeline> {
    let mut cache_data: Cache = match use_caches {
        true => validate_caches(vulkan),
        false => Cache::default()
    };

    let thread_count = available_parallelism()
        .map(|x| x.get())
        .map(|x| max(x, pipeline_infos.len()))
        .unwrap_or(1);

    let mut main_pool = CachePool::new(&[], vulkan.clone());
    let pipelines = std::thread::scope(|s| {
        let handles: Vec<_> = pipeline_infos.as_slice()
            .chunks(thread_count)
            .map(|info_chunk| {
                let mut cache_pool = CachePool::new(&cache_data.cache_blob, vulkan.clone());
                s.spawn(move || {
                    let cache = cache_pool.pull();
                    let pipelines = vulkan.create_graphic_pipelines(info_chunk, cache);
                    cache_pool.ret(cache);
                    
                    (pipelines, cache_pool)
                })
            })
            .collect();

        let mut pipelines = Vec::with_capacity(handles.len());
        for handle in handles {
            let (mut pipeline_vec, cache_pool) = handle.join().unwrap();
            main_pool.merge(cache_pool);
            pipelines.append(&mut pipeline_vec);
        }
        pipelines
    });

    let final_cache = main_pool.yield_result();
    if use_caches {
        cache_data.cache_blob = vulkan.get_data_from_pipeline_cache(final_cache);
        let proto_bytes = cache_data.encode_to_vec();
        let compressed_data = compression_algo(&proto_bytes);

        fs::write(FILE_PATH, compressed_data.as_slice()).expect("Unable to write file");
    }
    final_cache.destroy(vulkan);

    pipelines
}

fn validate_caches(vulkan: &Vulkan) -> Cache {
    match File::open(FILE_PATH) {
        Ok(_) => {
            let raw_bytes = fs::read(FILE_PATH).expect("Unable to read file");
            let decoded_bytes = decompression_algo(&raw_bytes);
            let cache_data = Cache::decode(decoded_bytes.as_slice()).expect("Unable to decode cache");

            if cache_data.device == None || cache_data.cache_blob.len() == 0 || cache_data.version != VERSION {
                return devalidate(vulkan);
            }

            if cache_data.device.unwrap() != build_device_info(vulkan.get_loaded_device()) {
                return devalidate(vulkan);
            }

            cache_data
        },
        Err(_) => {
            create_new_cache(vulkan)
        },
    }
}
#[inline(always)]
fn devalidate(vulkan: &Vulkan) -> Cache {
    #[cfg(debug_assertions)] println!("Cache devalidated");
    create_new_cache(vulkan)
}

#[inline(always)]
pub fn compression_algo(bytes: &[u8]) -> Vec<u8> {
    lz4_flex::compress_prepend_size(bytes)
}

#[inline(always)]
pub fn decompression_algo(bytes: &[u8]) -> Vec<u8> {
    lz4_flex::decompress_size_prepended(bytes).unwrap_or_else(|err|  panic!("{}", err))
}

pub fn create_new_cache(vulkan: &Vulkan) -> Cache {
    let mut cache_data = Cache::default();
    cache_data.version = VERSION;
    cache_data.device = Some(build_device_info(vulkan.get_loaded_device()));

    cache_data
}

struct CachePool {
    caches: Vec<VkPipelineCache>,
    cache_blob: Arc<[u8]>,
    vulkan: Arc<Vulkan>,
}

impl CachePool {
    pub fn new(cache_blob: &[u8], vulkan: Vulkan) -> Self {
        Self {
            caches: vec![],
            cache_blob: Arc::from(cache_blob),
            vulkan: Arc::new(vulkan),
        }
    }

    pub fn merge(&mut self, other: CachePool) {
        self.caches.extend(other.caches);
    }
    pub fn pull(&mut self) -> VkPipelineCache {
        self.caches.pop().unwrap_or_else(|| self.vulkan.create_pipeline_cache(&self.cache_blob))
    }

    pub fn ret(&mut self, cache: VkPipelineCache) {
        self.caches.push(cache);
    }

    pub fn yield_result(mut self) -> VkPipelineCache {
        let dst_cache = self.caches.pop().unwrap();
        if !self.caches.is_empty() {
            self.vulkan.merge_pipeline_caches(self.caches, &dst_cache);
        }
        dst_cache
    }
}

