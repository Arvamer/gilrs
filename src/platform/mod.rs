pub use self::platform::*;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;
