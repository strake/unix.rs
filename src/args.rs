use str::*;

pub struct Args(*const *const u8);

impl Args {
    #[inline] pub unsafe fn from_c_argv(c_argv: *const *const u8) -> Self { Args(c_argv) }
}

impl Iterator for Args {
    type Item = &'static OsStr;
    #[inline] fn next(&mut self) -> Option<&'static OsStr> { unsafe {
        if (*self.0).is_null() { None } else {
            let ret = OsStr::from_ptr(*self.0);
            self.0 = self.0.offset(1);
            Some(ret)
        }
    } }
}
