use std::{ffi::{c_char, CStr}, mem::ManuallyDrop, ptr};

#[repr(C)]
pub struct ByteBuffer {
    ptr: *mut u8,
    length: i32,
    capacity: i32,
}

impl ByteBuffer {
    /// Create a ByteBuffer from a Vec<u8>. If empty, ptr=null, length=0, capacity=0.
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return Self { ptr: ptr::null_mut(), length: 0, capacity: 0 };
        }
        let length = i32::try_from(bytes.len()).expect("length out of range");
        let capacity = i32::try_from(bytes.capacity()).expect("capacity out of range");

        let mut v = ManuallyDrop::new(bytes);
        Self {
            ptr: v.as_mut_ptr(),
            length,
            capacity,
        }
    }

    /// Create a ByteBuffer from a Vec<T>, storing total bytes in `length`/`capacity`.
    pub fn from_vec_struct<T>(bytes: Vec<T>) -> Self {
        if bytes.is_empty() {
            return Self { ptr: ptr::null_mut(), length: 0, capacity: 0 };
        }
        let elem_size = std::mem::size_of::<T>();
        let length = i32::try_from(bytes.len() * elem_size).expect("length out of range");
        let capacity = i32::try_from(bytes.capacity() * elem_size).expect("capacity out of range");

        let mut v = ManuallyDrop::new(bytes);
        Self {
            ptr: v.as_mut_ptr() as *mut u8,
            length,
            capacity,
        }
    }

    /// Get the raw pointer as *mut u8 (panics if `ptr` is invalid).
    pub fn ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Number of bytes (panics if negative or overflowed).
    pub fn len(&self) -> usize {
        self.length.try_into().expect("length < 0 or overflowed")
    }

    /// Capacity in bytes (panics if negative or overflowed).
    pub fn cap(&self) -> usize {
        self.capacity.try_into().expect("capacity < 0 or overflowed")
    }

    /// Return a slice view of the buffer as &[T]. If null or length=0, returns an empty slice.
    pub fn into_slice<T>(&self) -> &[T] {
        if self.ptr.is_null() || self.length == 0 {
            &[]
        } else {
            let count = self.len() / std::mem::size_of::<T>();
            unsafe { std::slice::from_raw_parts(self.ptr as *const T, count) }
        }
    }

    /// Interpret the buffer as a null-terminated string array, returning Vec<String>.
    /// If null or length=0, returns an empty Vec.
    pub fn into_string_vec(&self) -> Vec<String> {
        if self.ptr.is_null() || self.length == 0 {
            Vec::new()
        } else {
            self.into_slice::<*mut c_char>()
                .iter()
                .map(|&cstr_ptr| {
                    unsafe { CStr::from_ptr(cstr_ptr) }
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        }
    }

    /// Destroy this buffer, converting to Vec<u8>. If null or length=0, returns an empty Vec.
    pub fn destroy_into_vec(self) -> Vec<u8> {
        if self.ptr.is_null() || self.length <= 0 {
            Vec::new()
        } else {
            let cap = self.cap();
            let len = self.len();
            unsafe { Vec::from_raw_parts(self.ptr, len, cap) }
        }
    }

    /// Destroy this buffer, converting to Vec<T>. If null or length=0, returns an empty Vec.
    pub fn destroy_into_vec_struct<T>(self) -> Vec<T> {
        if self.ptr.is_null() || self.length <= 0 {
            Vec::new()
        } else {
            let elem_size = std::mem::size_of::<T>();
            let len_bytes = self.len();
            let cap_bytes = self.cap();
            let count = len_bytes / elem_size;
            let cap_count = cap_bytes / elem_size;
            unsafe { Vec::from_raw_parts(self.ptr as *mut T, count, cap_count) }
        }
    }

    /// Destroy this buffer by dropping its contents. If null or length=0, does nothing.
    pub fn destroy(self) {
        drop(self.destroy_into_vec());
    }
}