
#[macro_export]
macro_rules! make_parse_collect_slice_fn {
    ($name:ident, $ty:ty) => {
        fn $name<Output: Clone, F>(input: *mut Slice<$ty>, parser: F) -> Vec<Output>  
        where F: Fn(&$ty) -> Output, {
            unsafe {
                if !input.is_null() && !(*input).ptr.is_null() && (*input).len > 0 {
                    let slice = std::slice::from_raw_parts((*input).ptr, (*input).len);
                    slice.iter()
                        .map(|ptr| parser(ptr)) 
                        .collect::<Vec<_>>()
                } else {
                    panic!("Invalid pointer passed to {}!", stringify!($name));
                }
            }
        }
    }
}

#[macro_export]
macro_rules! make_parse_slice_fn {
    ($name:ident, $ty:ty) => {
        fn $name(input: *mut Slice<$ty>) -> Vec<$ty> {
            unsafe {
                if !input.is_null() && !(*input).ptr.is_null() && (*input).len > 0 {
                    let slice = std::slice::from_raw_parts((*input).ptr, (*input).len);
                    slice.iter().cloned().collect::<Vec<_>>()
                } else {
                    panic!("Invalid pointer passed to {}!", stringify!($name));
                }
            }
        }
    }
}