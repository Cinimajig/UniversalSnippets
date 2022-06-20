use std::{io, ptr};

use winapi::{shared::{windef::HWND, ntdef::HANDLE}, um::{winuser::{CloseClipboard, OpenClipboard}, winbase::{GlobalAlloc, GMEM_MOVEABLE, GlobalUnlock, GlobalLock}}};

use crate::{last_os_err, clip_formats, utils::wstring};

pub struct Clipboard {
    owner: HWND,
    buffer: HANDLE,
}

impl Clipboard {
    pub fn new(window: HWND) -> io::Result<Self> {
        unsafe {
            for _ in 0..10 {
                if OpenClipboard(window) != 0 {
                    return Ok(Self { owner: window, buffer: ptr::null_mut() });
                }
            }

            Err(last_os_err!())
        }
    }

    pub fn alloc_buffer(&mut self, size: usize) {
        unsafe {
        }
    }

    pub fn set_clipboard(&self, format: clip_formats::Format, data: &str) {
        unsafe {
            let wstring: Vec<u16> = wstring(data);
            self.buffer = GlobalAlloc(GMEM_MOVEABLE, wstring.len() * 2);
            GlobalLock(self.handle);

            ptr::copy(wstring.as_ptr(), self.buffer as _, wstring.len());

            GlobalUnlock(self.handle);
        }
    } 
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        unsafe {
            GlobalUnlock(self.handle);
            CloseClipboard();
        }
    }
}