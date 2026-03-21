use amalgam::vulkan::func::{Destructible, Vulkan};
use amalgam::vulkan::utils::BufferUsage;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use amalgam::prelude::Allocator;

fn bench_allocation(c: &mut Criterion) {
    let mut vulkan = Vulkan::default();
    vulkan.init();

    let mut group = c.benchmark_group("allocator");

    let size = 100;
    group.bench_with_input(BenchmarkId::new("alloc/alloc_n_cleanup", size), &size, |b, &size| {
        b.iter(|| {
            let buffer = vulkan.create_buffer(size, BufferUsage::preset_staging()).unwrap();
            let _ = vulkan.allocator.device(vec![black_box(buffer)], &vulkan);
            buffer.destroy(&vulkan);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_allocation);
criterion_main!(benches);