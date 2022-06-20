#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clip_formats;
mod shortcuts;
mod utils;
mod windowing;

use clip_formats::Format;
use std::{env, fs, io, mem, path::Path, ptr};
use utils::wstring;
use winapi::{
    shared::{
        minwindef::{HIBYTE, HIWORD, LOBYTE, LOWORD},
        windef::{HWND, RECT},
    },
    um::{
        commctrl::{
            InitCommonControlsEx, HKM_GETHOTKEY, HKM_SETHOTKEY, HOTKEY_CLASS, ICC_HOTKEY_CLASS,
            INITCOMMONCONTROLSEX,
        },
        handleapi::CloseHandle,
        libloaderapi::GetModuleHandleW,
        processthreadsapi::OpenProcess,
        psapi::GetModuleFileNameExW,
        wingdi::TextOutW,
        winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        winuser::*,
    },
};
use windowing::*;

const MY_MAX_PATH: usize = 4096;
const SAVE_FOLDER: &str = "usnip";
const WND_CLASS: &str = "usnip_";
const WND_TITLE: &str = "Universal snipets (alfa)";
const WND_WIDTH: i32 = 350;
const WND_HEIGHT: i32 = 250;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        return;
    }
}

fn run() -> io::Result<()> {
    // Change later with error handling (if let)
    let path = create_save_folder().unwrap();

    Window::new(WND_CLASS, MainWindow::new(path))?.build(
        WND_TITLE,
        WND_WIDTH,
        WND_HEIGHT,
        WS_VISIBLE | WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_MINIMIZEBOX,
    )?;

    msg_loop();
    Ok(())
}

fn create_save_folder() -> Result<String, env::VarError> {
    let path = format!(
        "{}{}\\{SAVE_FOLDER}",
        env::var("HOMEDRIVE")?,
        env::var("HOMEPATH")?
    );

    if !Path::new(path.as_str()).is_dir() {
        fs::create_dir(&path).unwrap_or_else(|err| unsafe {
            let text: Vec<u16> = err
                .to_string()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            MessageBoxW(ptr::null_mut(), text.as_ptr(), ptr::null(), MB_ICONERROR);
        });
    }

    Ok(path)
}

fn msg_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            if msg.message == WM_QUIT {
                break;
            }

            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Fixes a bug with `HKM_GETHOTKEY` switching ALT and SHIFT arround.
fn fix_modifers(mods: u8) -> u8 {
    let shift = mods & 0x1;
    let alt = mods & 0x4;

    // If both, then nothing is wrong.
    if shift == 0x1 && alt == 0x4 {
        return mods;
    }

    if shift == 0x1 {
        return (mods ^ 0x1) ^ 0x4;
    }

    if alt == 0x4 {
        return (mods ^ 0x4) ^ 0x1;
    }

    mods
}

fn get_focus() -> HWND {
    use winapi::um::processthreadsapi::GetCurrentThreadId;

    let mut result = 0;
    let mut tid = 0;
    let mut pid = 0;
    
    unsafe {
        result = GetFocus();

        if result.is_null() {
            let window = GetForegroundWindow();
            if !window.is_null() {
                tid = GetWindowThreadProcessId(window, &mut pid);
                if AttachThreadInput(GetCurrentThreadId(), tid, 1) != 0 {
                    result = GetFocus();
                    AttachThreadInput(GetCurrentThreadId(), tid, 0);
                }
            }
        }
    }

    result
}

fn find_process(window: HWND) -> String {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(window, &mut pid);

        if pid == 0 {
            return "".to_string();
        }

        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);

        if handle.is_null() {
            return "".to_string();
        }

        let mut buffer = [0_u16; MY_MAX_PATH];
        let size = GetModuleFileNameExW(
            handle,
            ptr::null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        );

        CloseHandle(handle);

        let mut ps = String::from_utf16_lossy(&buffer[..size as usize]);
        let ps_name = ps.split('\\').last().unwrap();

        ps_name.to_lowercase()
    }
}

fn paste_data(window: HWND, data: &String) {
    use clipboard_win::*;

    if let Ok(_clip) = Clipboard::new_attempts(10) {
        println!("ACCESS!");
        
        let mut old_clip = String::new();
        formats::Unicode.read_clipboard(&mut old_clip).unwrap();

        formats::Unicode.write_clipboard(data).unwrap();

        unsafe {
            let mut ip = INPUT {
                type_: INPUT_KEYBOARD,
                ..Default::default()
            };

            {
                let ki = ip.u.ki_mut();
                ki.dwFlags = 0;
                ki.time = 0;
                ki.wVk = VK_CONTROL as u16;
                SendInput(1, &mut ip, mem::size_of::<INPUT>() as i32);
            }

            {
                let ki = ip.u.ki_mut();
                ki.dwFlags = 0;
                ki.wVk = 0x56; // V
                SendInput(1, &mut ip, mem::size_of::<INPUT>() as i32);
            }

            {
                let ki = ip.u.ki_mut();
                ki.wVk = 0x56;
                ki.dwFlags = KEYEVENTF_KEYUP;
                SendInput(1, &mut ip, mem::size_of::<INPUT>() as i32);
            }

            {
                let ki = ip.u.ki_mut();
                ki.wVk = VK_CONTROL as u16;
                ki.dwFlags = KEYEVENTF_KEYUP;
                SendInput(1, &mut ip, mem::size_of::<INPUT>() as i32);
            }
        }

        formats::Unicode.write_clipboard(&old_clip).unwrap();
    }
}

struct Shortcut {
    id: i32,
    version: String,
    process: String,
    enabled: bool,
    modifiers: u16,
    keycode: u16,
    format: Format,
    data: String,
}

struct MainWindow {
    path: String,
    handle: HWND,
    h_hotkey: HWND,
    last_keys: (u8, u8),
    register: Vec<Shortcut>,
}

impl MainWindow {
    fn new(path: String) -> Self {
        Self {
            path,
            h_hotkey: ptr::null_mut(),
            handle: ptr::null_mut(),
            last_keys: (0, 0),
            register: vec![],
        }
    }

    fn register_hotkeys(&mut self) {
        if let Ok(files) = fs::read_dir(self.path.as_str()) {
            for (index, file) in files.enumerate() {
                if let Ok(entry) = file {
                    let path = entry.path();

                    if !path.is_file() {
                        continue;
                    }

                    if let Ok(content) = fs::read_to_string(&path) {
                        let split: Vec<&str> = content.splitn(7, ';').collect();

                        // TODO! Safer indexing.
                        self.register.push(Shortcut {
                            id: index as i32,
                            version: split[0].to_string(),
                            process: split[1].to_lowercase(),
                            enabled: split[2].parse().unwrap_or(true),
                            modifiers: split[3].parse().unwrap_or_default(),
                            keycode: split[4].parse().unwrap_or(0x87),
                            format: split[5].parse().unwrap_or_default(),
                            data: split[6].to_string(),
                        });
                    }
                }
            }
        }

        for item in &self.register {
            unsafe {
                println!(
                    "{:?} => mod: {}, key: {}",
                    self.handle, item.modifiers, item.keycode
                );
                RegisterHotKey(
                    self.handle,
                    item.id,
                    item.modifiers as u32,
                    item.keycode as u32,
                );
            }
        }
    }
}

impl WindowProc for MainWindow {
    fn window_proc(
        &mut self,
        window: HWND,
        msg: u32,
        wparam: winapi::shared::minwindef::WPARAM,
        lparam: winapi::shared::minwindef::LPARAM,
    ) -> Option<winapi::shared::minwindef::LRESULT> {
        let mut result = Some(0);

        unsafe {
            match msg {
                WM_CREATE => {
                    self.handle = window;
                    self.register_hotkeys();

                    let icex = INITCOMMONCONTROLSEX {
                        dwSize: mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                        dwICC: ICC_HOTKEY_CLASS,
                    };
                    InitCommonControlsEx(&icex);

                    let whotkey_class = wstring(HOTKEY_CLASS);

                    self.h_hotkey = CreateWindowExW(
                        0,
                        whotkey_class.as_ptr(),
                        ptr::null(),
                        WS_CHILD | WS_VISIBLE,
                        15,
                        10,
                        200,
                        20,
                        window,
                        ptr::null_mut(),
                        GetModuleHandleW(ptr::null_mut()),
                        ptr::null_mut(),
                    );
                }
                WM_COMMAND => {
                    match HIWORD(wparam as u32) {
                        EN_CHANGE => {
                            let result = SendMessageW(self.h_hotkey, HKM_GETHOTKEY, 0, 0);
                            let keycode = LOBYTE(LOWORD(result as u32) as u16);
                            let modifiers = fix_modifers(HIBYTE(LOWORD(result as u32) as u16));
                            self.last_keys = (modifiers, keycode);

                            let mut rect = RECT::default();
                            GetClientRect(self.handle, &mut rect);
                            InvalidateRect(self.handle, &rect, 1);
                        }
                        _ => (),
                    };
                }
                WM_PAINT => {
                    let mut ps = PAINTSTRUCT::default();
                    let mut rect = RECT::default();
                    GetClientRect(window, &mut rect);

                    let dc = BeginPaint(window, &mut ps);

                    let text = format!(
                        "Keycode: {}, Modifier: {}",
                        self.last_keys.1, self.last_keys.0
                    );
                    let wtext = wstring(&text);
                    TextOutW(dc, 15, 40, wtext.as_ptr(), wtext.len() as i32);

                    EndPaint(window, &ps);
                }
                WM_HOTKEY => {
                    let modifiers = LOWORD(lparam as u32);
                    let keycode = HIWORD(lparam as u32);
                    let foreground_window = get_focus();

                    for hotkey in &self.register {
                        if !hotkey.enabled {
                            continue;
                        }

                        if hotkey.modifiers == modifiers && hotkey.keycode == keycode {
                            if foreground_window == window {
                                let text = format!(
                                    "Hotkey is already setup for process: {}\0",
                                    &hotkey.process
                                );

                                MessageBoxA(
                                    window,
                                    text.as_ptr() as _,
                                    b"Hotkey already in use\0".as_ptr() as _,
                                    MB_ICONINFORMATION,
                                );
                            }

                            if hotkey.process != find_process(foreground_window) {
                                continue;
                            }

                            paste_data(&hotkey.data);
                            break;
                        }
                    }
                }
                WM_DESTROY => {
                    for shortcut in &self.register {
                        UnregisterHotKey(window, shortcut.id);
                    }
                    PostQuitMessage(0);
                }
                _ => result = None,
            };

            result
        }
    }
}
