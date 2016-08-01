pub use self::platform::*;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;

#[cfg(all(not(target_os = "linux"), not(target_os = "windows")))]
#[path = "default/mod.rs"]
mod platform;
