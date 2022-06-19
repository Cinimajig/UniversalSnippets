#[macro_export]
macro_rules! last_os_err {
    () => {
        ::std::io::Error::last_os_error()
    };
}

#[inline(always)]
pub fn wstring(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

