pub use self::platform::*;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(all(not(target_os = "linux")))]
#[path = "default/mod.rs"]
mod platform;
