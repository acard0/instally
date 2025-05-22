use std::{ffi::{c_char, CStr}, cmp::Ordering};

use instally_core::{definitions::{package::Package, summary::PackagePair}, helpers::{like::CStringLike, versioning::version_compare}, definitions::context::AppContext};

#[repr(C)]
pub struct CallResult<T> {
    pub result: T,
    pub error: *const c_char,
}

impl<T> CallResult<T> {
    pub fn new(result: T, error: Option<&str>) -> Self {
        let err = match error {
            None => std::ptr::null(),
            Some(s) => s.as_c_char_ptr()
        };

        CallResult { 
            result,
            error: err
        }
    }

    pub fn into_raw(self) -> *mut CallResult<T> {
        Box::into_raw(Box::new(self))
    }   
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CPackageVersioning {
    name: *const i8,
    display_name: *const i8,
    v_current: *const i8,
    v_latest: *const i8,
    default: i32,
    state: i32,
}

impl CPackageVersioning {
    pub fn new(cross: &PackagePair) -> Self {
        CPackageVersioning {
            name: cross.local.name.as_c_char_ptr(),
            display_name: cross.local.display_name.as_c_char_ptr(),
            v_current: cross.local.version.as_c_char_ptr(),
            v_latest: cross.remote.version.as_c_char_ptr(),
            default: cross.remote.default as i32,
            state: match version_compare(&cross.remote.version, &cross.local.version) {
                Ordering::Greater => 1,
                Ordering::Less => -1,
                Ordering::Equal => 0,
            }
        }       
    }

    pub fn new_not_installed(remote: &Package) -> Self {
        CPackageVersioning {
            name: remote.name.as_c_char_ptr(),
            display_name: remote.display_name.as_c_char_ptr(),
            v_current: "0".as_c_char_ptr(),
            v_latest: remote.version.as_c_char_ptr(),
            default: remote.default as i32,
            state: -2,
        }
    }

    pub fn get_name(&self) -> String {
        unsafe { CStr::from_ptr(self.name).to_str().unwrap().to_string() }
    }

    pub fn get_v_current(&self) -> String {
        unsafe { CStr::from_ptr(self.v_current).to_str().unwrap().to_string() }
    }

    pub fn get_v_latest(&self) -> String {
        unsafe { CStr::from_ptr(self.v_latest).to_str().unwrap().to_string() }
    }

    pub fn get_outdated(&self) -> bool {
        self.state == 1
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CAppState { // TODO: caller side buffer
    pub state: *const i8,
    pub state_progress: f32,
    pub result: *const i8,
}

impl From<AppContext> for CAppState {
    fn from(value: AppContext) -> Self {
        CAppState {
            state_progress: value.get_progress(),
            state: value.get_state().map(|s| s.as_c_char_ptr()).unwrap_or(std::ptr::null_mut::<i8>()),
            result: value.get_result().map(|s| format!("{s:?}").as_c_char_ptr()).unwrap_or(std::ptr::null_mut::<i8>()),
        }
    }
}