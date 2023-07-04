
pub trait CStringLike {
    fn as_c_char_ptr(&self) -> *mut std::ffi::c_char; 
}

impl CStringLike for String {
    fn as_c_char_ptr(&self) -> *mut std::ffi::c_char {
        c_str(self)
    }
}

impl CStringLike for &str {
    fn as_c_char_ptr(&self) -> *mut std::ffi::c_char {
        c_str(self)
    }
}

fn c_str(str: &str) -> *mut std::ffi::c_char {
    std::ffi::CString::new(str).unwrap().into_raw()
}