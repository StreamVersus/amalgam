use crate::engine::caches::{build_device_info, Cache};
use crate::vulkan::func::{Destructible, Vulkan};
use crate::vulkan::r#impl::pipelines::GraphicsPipelineCreateInfo;
use prost::Message;
use std::fs;
use std::fs::File;
use vulkan_raw::{VkPipeline, VkPipelineCache};

const FILE_PATH: &str = "cache.storage";
const VERSION: u32 = 0;

pub fn create_pipelines_multithreaded(use_caches: bool, pipeline_infos: Vec<GraphicsPipelineCreateInfo>, vulkan: &Vulkan) -> Vec<VkPipeline> {
    let mut cache_data: Cache;

    if use_caches {
        match File::open(FILE_PATH) {
            Ok(_) => {
                let raw_bytes = fs::read(FILE_PATH).expect("Unable to read file");
                let decoded_bytes = decompression_algo(&raw_bytes);
                cache_data = Cache::decode(decoded_bytes.as_slice()).expect("Unable to decode cache");

                if cache_data.device == None || cache_data.cache_blob.len() == 0 || cache_data.version != VERSION {
                    #[cfg(debug_assertions)] println!("Cache devalidated");
                    cache_data = create_new_cache(vulkan);
                }

                if cache_data.device.unwrap() != build_device_info(vulkan.get_loaded_device()) {
                    #[cfg(debug_assertions)] println!("Cache devalidated");
                    cache_data = create_new_cache(vulkan);
                };
            },
            Err(_) => {
                cache_data = create_new_cache(vulkan);
            },
        }
    } else {
        cache_data = Cache::default();
    };

    let mut caches: Vec<VkPipelineCache> = Vec::with_capacity(pipeline_infos.len());
    for _ in pipeline_infos.iter() {
        caches.push(vulkan.create_pipeline_cache(cache_data.cache_blob.as_slice()));
    };
    let pipelines = std::thread::scope(|s| {
        let handles: Vec<_> = pipeline_infos.into_iter()
            .zip(caches.iter())
            .map(|(info, &cache)| {
                let vulkan = &vulkan;
                s.spawn(move || {
                    vulkan.create_graphic_pipelines(vec![info], cache)
                })
            })
            .collect();

        let mut pipelines = Vec::new();
        for handle in handles {
            let mut pipeline_vec = handle.join().unwrap();
            pipelines.append(&mut pipeline_vec);
        }
        pipelines
    });

    let dst_cache = caches.pop().unwrap();
    if !caches.is_empty() {
        vulkan.merge_pipeline_caches(caches, &dst_cache);
    };

    if use_caches {
        cache_data.cache_blob = vulkan.get_data_from_pipeline_cache(dst_cache);
        let proto_bytes = cache_data.encode_to_vec();
        let compressed_data = compression_algo(&proto_bytes);

        fs::write(FILE_PATH, compressed_data.as_slice()).expect("Unable to write file");
    }
    dst_cache.destroy(vulkan);

    pipelines
}
pub fn compression_algo(bytes: &[u8]) -> Vec<u8> {
    lz4_flex::compress_prepend_size(bytes)
}

pub fn decompression_algo(bytes: &[u8]) -> Vec<u8> {
    lz4_flex::decompress_size_prepended(bytes).unwrap_or_else(|err|  panic!("{}", err))
}

pub fn create_new_cache(vulkan: &Vulkan) -> Cache {
    let mut cache_data = Cache::default();
    cache_data.version = VERSION;
    cache_data.device = Some(build_device_info(vulkan.get_loaded_device()));

    cache_data
}
