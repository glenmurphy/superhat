use std::ffi::CString;
use windows::Win32::Foundation::{HANDLE, HWND, BOOL, CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowA, SetForegroundWindow, ShowWindow, 
    SetWindowLongA, GetWindowLongA, SW_RESTORE,
    GWL_STYLE, WS_MAXIMIZEBOX, WS_SIZEBOX, WINDOW_STYLE,
    SB_BOTH, SetWindowTextA,
};
use windows::Win32::UI::Controls::ShowScrollBar;
use windows::Win32::System::Console::GetConsoleWindow;
use windows::core::{PCSTR, PCWSTR};
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
            let handle = CreateMutexW(
                None,
                BOOL::from(false),
                PCWSTR::from_raw(wide_name.as_ptr()),
            ).expect("Failed to create mutex");
            
            if handle == HANDLE(0) {
                return None; // Failed to create mutex
            }
            
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle).expect("Failed to close handle");

                // Find and refocus the existing window
                let window_name = CString::new(window_title).unwrap();
                let existing_window = FindWindowA(
                    PCSTR::null(),
                    PCSTR::from_raw(window_name.as_ptr() as *const u8),
                );
                
                if existing_window != HWND(0) {
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
            if hwnd != HWND(0) {
                let current_style = GetWindowLongA(hwnd, GWL_STYLE);
                let new_style = WINDOW_STYLE(
                    (current_style as u32) & !(WS_MAXIMIZEBOX.0 | WS_SIZEBOX.0)
                );
                
                SetWindowLongA(hwnd, GWL_STYLE, new_style.0 as i32);
                ShowScrollBar(hwnd, SB_BOTH, false);
                
                let title = CString::new(window_title).unwrap();
                SetWindowTextA(hwnd, PCSTR::from_raw(title.as_ptr() as *const u8));
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
            if self.mutex_handle != HANDLE(0) {
                CloseHandle(self.mutex_handle).expect("Failed to close mutex handle");
            }
        }
    }
} 