use std::{ffi::{c_char, CStr}, cmp::Ordering};

use instally_core::{workloads::{updater::{PackagePair}, abstraction::{AppContext}}, helpers::{versioning::version_compare, like::CStringLike}};

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
    v_current: *const i8,
    v_latest: *const i8,
    outdated: bool
}

impl CPackageVersioning {
    pub fn new(cross: &PackagePair) -> Self {
        CPackageVersioning {
            name: cross.local.name.as_c_char_ptr(),
            v_current: cross.local.version.as_c_char_ptr(),
            v_latest: cross.remote.version.as_c_char_ptr(),
            outdated: version_compare(&cross.remote.version, &cross.local.version) == Ordering::Greater,
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
        self.outdated
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CAppState {
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

//////////////////////////////////////////////////////////////////////////////////////////////////
/// Buffer impl. Taken from csbindgen repo
/// //////////////////////////////////////////////////////////////////////////////////////////////

#[repr(C)]
pub struct ByteBuffer {
    ptr: *mut u8,
    length: i32,
    capacity: i32,
}

impl ByteBuffer {
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
            .try_into()
            .expect("invalid pointer")
    }

    pub fn cap(&self) -> usize {
        self.capacity
            .try_into()
            .expect("buffer cap negative or overflowed")
    }

    pub fn len(&self) -> usize {
        self.length
            .try_into()
            .expect("buffer length negative or overflowed")
    }

    pub fn from_vec(bytes: Vec<u8>) -> Self {
        let length = i32::try_from(bytes.len()).expect("buffer length cannot fit into a i32.");
        let capacity =
            i32::try_from(bytes.capacity()).expect("buffer capacity cannot fit into a i32.");

        let mut v = std::mem::ManuallyDrop::new(bytes);

        Self {
            ptr: v.as_mut_ptr(),
            length,
            capacity,
        }
    }

    pub fn from_vec_struct<T: Sized>(bytes: Vec<T>) -> Self {
        let element_size = std::mem::size_of::<T>() as i32;

        let length = (bytes.len() as i32) * element_size;
        let capacity = (bytes.capacity() as i32) * element_size;

        let mut v = std::mem::ManuallyDrop::new(bytes);

        Self {
            ptr: v.as_mut_ptr() as *mut u8,
            length,
            capacity,
        }
    }

    pub fn into_slice<T: Sized>(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr() as *mut T, self.len() / std::mem::size_of::<T>()) }
    }

    pub fn into_string_vec(&self) -> Vec<String> {
        self.into_slice::<*mut c_char>()
        .iter().map(|f| unsafe { CStr::from_ptr(*f).to_str().unwrap().to_string() })
        .collect::<Vec<_>>()
    }
    
    pub fn destroy_into_vec(self) -> Vec<u8> {
        if self.ptr.is_null() {
            vec![]
        } else {
            let capacity: usize = self
                .capacity
                .try_into()
                .expect("buffer capacity negative or overflowed");
            let length: usize = self
                .length
                .try_into()
                .expect("buffer length negative or overflowed");

            unsafe { Vec::from_raw_parts(self.ptr, length, capacity) }
        }
    }

    pub fn destroy_into_vec_struct<T: Sized>(self) -> Vec<T> {
        if self.ptr.is_null() {
            vec![]
        } else {
            let element_size = std::mem::size_of::<T>() as i32;
            let length = (self.length * element_size) as usize;
            let capacity = (self.capacity * element_size) as usize;

            unsafe { Vec::from_raw_parts(self.ptr as *mut T, length, capacity) }
        }
    }

    pub fn destroy(self) {
        drop(self.destroy_into_vec());
    }
}

