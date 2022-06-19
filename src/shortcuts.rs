#![allow(unused_variables, dead_code)]
use std::{rc::Rc, io, mem, ptr};
use winapi::{um::{winuser::*, libloaderapi::GetModuleHandleW}, ctypes::c_void, shared::windef::HWND};
use crate::{windowing::*, utils::*, last_os_err};


