use libudev_sys as ud;
use std::ffi::{CStr, CString};

#[derive(Debug)]
pub struct Udev(*mut ud::udev);

impl Udev {
    pub fn new() -> Option<Self> {
        let u = unsafe { ud::udev_new() };
        if u.is_null() {
            None
        } else {
            Some(Udev(u))
        }
    }

    pub fn enumerate(&self) -> Option<Enumerate> {
        let en = unsafe { ud::udev_enumerate_new(self.0) };
        if en.is_null() {
            None
        } else {
            let en = Enumerate(en);
            Some(en)
        }
    }
}

impl Drop for Udev {
    fn drop(&mut self) {
        unsafe {
            ud::udev_unref(self.0);
        }
    }
}

impl Clone for Udev {
    fn clone(&self) -> Self {
        Udev(unsafe { ud::udev_ref(self.0) })
    }
}

pub struct Enumerate(*mut ud::udev_enumerate);

impl Enumerate {
    pub fn scan_devices(&self) {
        // TODO: Check for error
        let _ = unsafe { ud::udev_enumerate_scan_devices(self.0) };
    }

    pub fn add_match_property(&self, key: &CStr, val: &CStr) {
        // TODO: Check for error
        unsafe {
            ud::udev_enumerate_add_match_property(self.0, key.as_ptr(), val.as_ptr());
        }
    }


    pub fn iter(&self) -> DeviceIterator {
        DeviceIterator(unsafe { ud::udev_enumerate_get_list_entry(self.0) })
    }
}

impl Drop for Enumerate {
    fn drop(&mut self) {
        unsafe {
            ud::udev_enumerate_unref(self.0);
        }
    }
}

pub struct DeviceIterator(*mut ud::udev_list_entry);

impl Iterator for DeviceIterator {
    type Item = CString;

    fn next(&mut self) -> Option<CString> {
        if self.0.is_null() {
            None
        } else {
            let p_name = unsafe { ud::udev_list_entry_get_name(self.0) };
            let name = if p_name.is_null() {
                return None;
            } else {
                unsafe { CStr::from_ptr(p_name).to_owned() }
            };
            self.0 = unsafe { ud::udev_list_entry_get_next(self.0) };
            Some(name)
        }
    }
}

pub struct Device(*mut ud::udev_device);

impl Device {
    pub fn from_syspath(udev: &Udev, path: &CStr) -> Option<Self> {
        let dev = unsafe { ud::udev_device_new_from_syspath(udev.0, path.as_ptr()) };
        if dev.is_null() {
            None
        } else {
            Some(Device(dev))
        }
    }

    pub fn devnode(&self) -> Option<&CStr> {
        unsafe {
            let s = ud::udev_device_get_devnode(self.0);
            if s.is_null() {
                None
            } else {
                Some(CStr::from_ptr(s))
            }
        }
    }

    pub fn properties(&self) -> PropertyIterator {
        let prop = unsafe { ud::udev_device_get_properties_list_entry(self.0) };
        PropertyIterator(prop)
    }
}

impl Clone for Device {
    fn clone(&self) -> Self {
        unsafe { Device(ud::udev_device_ref(self.0)) }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            ud::udev_device_unref(self.0);
        }
    }
}

pub struct PropertyIterator(*mut ud::udev_list_entry);

impl Iterator for PropertyIterator {
    type Item = (String, String);

    fn next(&mut self) -> Option<(String, String)> {
        if self.0.is_null() {
            None
        } else {
            let p_name = unsafe { ud::udev_list_entry_get_name(self.0) };
            let p_val = unsafe { ud::udev_list_entry_get_value(self.0) };

            let name = if p_name.is_null() {
                return None;
            } else {
                unsafe { CStr::from_ptr(p_name).to_string_lossy().into_owned() }
            };

            let value = if p_val.is_null() {
                return None;
            } else {
                unsafe { CStr::from_ptr(p_val).to_string_lossy().into_owned() }
            };

            self.0 = unsafe { ud::udev_list_entry_get_next(self.0) };
            Some((name, value))
        }
    }
}
