use std::ffi::CString;
use winapi::um::synchapi::CreateMutexW;
use winapi::um::errhandlingapi::GetLastError;
use winapi::shared::winerror::ERROR_ALREADY_EXISTS;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::shared::ntdef::HANDLE;
use winapi::um::winuser::{
    FindWindowA, SetForegroundWindow, ShowWindow, SW_RESTORE,
    SetWindowLongA, GetWindowLongA, ShowScrollBar, SetWindowTextA,
    GWL_STYLE, WS_MAXIMIZEBOX, WS_SIZEBOX, SB_BOTH,
};
use winapi::um::wincon::GetConsoleWindow;
use std::io;

pub struct WindowInstance {
    pub mutex_handle: HANDLE,
}

impl WindowInstance {
    fn create_mutex_name(window_title: &str) -> Vec<u16> {
        format!("Global\\{}_mutex", window_title)
            .encode_utf16()
            .chain(Some(0))
            .collect()
    }

    fn check_existing_instance(window_title: &str) -> Option<HANDLE> {
        let wide_name = Self::create_mutex_name(window_title);
        unsafe {
            let handle = CreateMutexW(std::ptr::null_mut(), 0, wide_name.as_ptr());
            if handle == INVALID_HANDLE_VALUE {
                return None; // Failed to create mutex
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);

                // Find and refocus the existing window
                let window_name = CString::new(window_title).unwrap();
                let existing_window = FindWindowA(std::ptr::null(), window_name.as_ptr());
                if !existing_window.is_null() {
                    ShowWindow(existing_window, SW_RESTORE);
                    SetForegroundWindow(existing_window);
                }

                println!("{} is already running!", window_title);
                return None;
            }
            Some(handle)
        }
    }

    pub fn new(window_title: &str) -> io::Result<Self> {
        // Try to create mutex first
        let mutex_handle = Self::check_existing_instance(window_title)
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Application already running"))?;

        // Set window properties
        unsafe {
            let hwnd = GetConsoleWindow();
            if !hwnd.is_null() {
                SetWindowLongA(hwnd, GWL_STYLE, GetWindowLongA(hwnd, GWL_STYLE) & !(WS_MAXIMIZEBOX | WS_SIZEBOX) as i32);
                ShowScrollBar(hwnd, SB_BOTH as i32, 0);
                let title = CString::new(window_title).unwrap();
                SetWindowTextA(hwnd, title.as_ptr());
            }
        }

        Ok(WindowInstance {
            mutex_handle,
        })
    }
}

impl Drop for WindowInstance {
    fn drop(&mut self) {
        unsafe {
            if self.mutex_handle != INVALID_HANDLE_VALUE {
                CloseHandle(self.mutex_handle);
            }
        }
    }
} 