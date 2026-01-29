#![no_std]
#![allow(unexpected_cfgs)]
#![allow(unused_imports)]

mod material;
use material::*;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "spirv")] {
        mod gpu;
        pub use gpu::*;
    } else {
        mod cpu;
        pub use cpu::*;
    }
}
