use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub mod windows;
        pub use windows::*;
    } else if #[cfg(target_os = "linux")] {
        pub mod linux;
        pub use linux::*;
    } else if #[cfg(target_os = "android")] {
        mod android;
        pub use android::*;
    } else {
        compile_error!("Unsupported platform, unable to parse platform-specific extensions, consider adding it to src/vulkan/platform");
    }
}

mod all;
pub use all::*;
