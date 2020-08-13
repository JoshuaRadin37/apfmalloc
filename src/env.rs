use std::os::raw::c_char;
use crate::independent_collections::Array;
use std::ffi::{CStr};
use libc::getenv;

pub struct IndependentCString(&'static str, Array<c_char>);

impl IndependentCString {

    pub const fn new(string: &'static str) -> Self {
        Self(string, Array::new())
    }

    pub fn as_c_slice(&mut self) -> &[c_char] {
        if self.1.is_empty() {
            let len = self.0.len();
            let array = &mut self.1;
            array.reserve(len + 1);
            for c in self.0.chars() {
                let c_char: c_char = c as c_char;
                array.push(c_char);
            }
            array.push(0i8);
        }
        self.1.as_ref()

    }


}

pub fn get_env(var: &'static str) -> Option<&CStr> {
    let mut ind = IndependentCString::new(var);
    let slice = ind.as_c_slice();
    let as_c_char_ptr: *const c_char = slice.as_ptr();
    let env_value = unsafe {
        getenv(as_c_char_ptr)
    };
    if env_value.is_null() {
        None
    } else {
        unsafe {
            Some(CStr::from_ptr(env_value))
        }
    }
}

pub fn env_is_value(var: &'static str, value: &'static str) -> bool {
    let env = get_env(var);
    if env.is_none() {
        return false;
    }

    let env_value = env.unwrap();

    let mut check_value = IndependentCString::new(value);
    unsafe {
        env_value == CStr::from_ptr(check_value.as_c_slice().as_ptr())
    }
}

macro_rules! get_env_parse {
    ($name:ident, $type:ty) => {
        pub fn $name(var: &'static str) -> Option<$type> {
            if let Some(value) = get_env(var) {
                let value = value.to_str().unwrap();
                value.parse::<$type>().ok()
            } else {
                None
            }

        }
    };
}

get_env_parse!(get_env_as_u8, u8);
get_env_parse!(get_env_as_u16, u16);
get_env_parse!(get_env_as_u32, u32);
get_env_parse!(get_env_as_u64, u64);
get_env_parse!(get_env_as_usize, usize);


#[cfg(test)]
mod test {
    use crate::env::get_env;

    #[test]
    fn get_env_var() {
        let should_exist = get_env("PATH");
        assert!(should_exist.is_some());
        let c_str = should_exist.unwrap();
        println!("PATH = {:?}", should_exist.unwrap().to_str());
        let should_not_exist = get_env("BOOPTASDASD");
        assert!(should_not_exist.is_none());
    }
}

