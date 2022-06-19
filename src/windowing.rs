#![allow(unused_variables, dead_code)]

use std::{mem, rc::Rc, ptr, io};
use winapi::{
    shared::{minwindef::*, windef::*},
    um::{winuser::*, libloaderapi::GetModuleHandleW}, ctypes::c_void,
};

use crate::{last_os_err, utils::*};

// `None` == `DefWindowProcW`.
pub trait WindowProc {
    fn window_proc(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
        let mut result = Some(0);

        unsafe {
            match msg {
                WM_DESTROY => {
                    PostQuitMessage(0);
                }
                _ => result = None,
            };

            result
        }
    }
}

pub unsafe extern "system" fn rwp(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = &*(lparam as *const CREATESTRUCTW);
        let window_state_ptr = create_struct.lpCreateParams;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_state_ptr as isize);
    }
    let window_proc_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Box<dyn WindowProc>;
    let result = {
        if window_proc_ptr.is_null() {
            None
        } else {
            let reference = Rc::from_raw(window_proc_ptr);
            mem::forget(reference.clone());
            (*window_proc_ptr).window_proc(hwnd, msg, wparam, lparam)
        }
    };

    if msg == WM_NCDESTROY && !window_proc_ptr.is_null() {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        mem::drop(Rc::from_raw(window_proc_ptr));
    }
    result.unwrap_or_else(|| DefWindowProcW(hwnd, msg, wparam, lparam))
}

pub struct Window {
    proc: Rc<Box<dyn WindowProc>>,
    cls: Vec<u16>,
}

impl Window {
    pub fn new(class_name: &str, proc: impl WindowProc + 'static) -> io::Result<Self> {
        let cls = wstring(class_name);

        unsafe {
            let wc = WNDCLASSEXW {
                cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
                lpszClassName: cls.as_ptr(),
                lpfnWndProc: Some(rwp),
                hbrBackground: 15 as _,
                hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW),
                hIcon: LoadIconW(ptr::null_mut(), IDI_APPLICATION),
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };

            if RegisterClassExW(&wc) == 0 {
                return Err(last_os_err!());
            }


            Ok(
                Self {
                    proc: Rc::new(Box::new(proc)), cls
                }
            )
        }
    }

    pub fn build(self, title: &str, width: i32, height: i32, styles: u32) -> io::Result<HWND> {
        unsafe {
            let wtitle = wstring(title);
            let proc_ptr = Rc::into_raw(self.proc) as *mut c_void;

            let window = CreateWindowExW(
                0,
                self.cls.as_ptr(),
                wtitle.as_ptr(),
                styles,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                ptr::null_mut(),
                ptr::null_mut(),
                GetModuleHandleW(ptr::null()),
                proc_ptr,
            );

            if window.is_null() {
                mem::drop(Rc::from_raw(proc_ptr));
                return Err(last_os_err!());
            }

            Ok(window)
        }
        
    }
}