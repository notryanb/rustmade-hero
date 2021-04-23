
// #[cfg(windows)] extern crate winapi;

#![cfg(windows)]
// Let's put this so that it won't open the console
#![windows_subsystem = "windows"]

use std::{error::Error, mem};
use std::ptr::null_mut;
use winapi::{shared::minwindef::*, um::wingdi::{BLACKNESS, PatBlt}};
use winapi::shared::windef::*;
use winapi::um::libloaderapi::{GetModuleHandleW};
use winapi::um::winuser::*;

// Window procedure function to handle events
pub unsafe extern "system" fn window_proc(
    window: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut paint = PAINTSTRUCT { 
                hdc: null_mut(), 
                fErase: 0, 
                rcPaint: RECT { left: 0, right: 100, top: 0, bottom: 100 }, 
                fRestore: 0, 
                fIncUpdate: 0, 
                rgbReserved: [0; 32],
            };

            let device_context: HDC = BeginPaint(window, &mut paint);
            let x = paint.rcPaint.left;
            let y = paint.rcPaint.top;
            let height = paint.rcPaint.bottom - paint.rcPaint.top;
            let width = paint.rcPaint.right - paint.rcPaint.left;
            PatBlt(device_context, x, y, width, height, BLACKNESS);
            EndPaint(window, &paint);
        }
        _ => return DefWindowProcW(window, msg, wparam, lparam),
    }
    return 0;
}


fn create_main_window(name: &str, title: &str) -> Result<HWND, Box<dyn Error>> {
    let name = to_wstring(name);
    let title = to_wstring(title);

    unsafe {
        let instance = GetModuleHandleW(null_mut());

        let window_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance, 
            hIcon: LoadIconW(null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: COLOR_WINDOWFRAME as HBRUSH,
            lpszMenuName: null_mut(),
            lpszClassName: name.as_ptr(),
            hIconSm: LoadIconW(null_mut(), IDI_APPLICATION),
        };

        if RegisterClassExW(&window_class) == 0 {
            MessageBoxW(
                null_mut(),
                to_wstring("Window Registration Failed!").as_ptr(),
                to_wstring("Error").as_ptr(),
                MB_ICONEXCLAMATION | MB_OK,
            );
            return Err("Window Registration Failed".into());
        }

        let window_handle = CreateWindowExW(
            0,
            name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        

        if window_handle.is_null() {
            MessageBoxW(
                null_mut(),
                to_wstring("Window Creation Failed!").as_ptr(),
                to_wstring("Error!").as_ptr(),
                MB_ICONEXCLAMATION | MB_OK,
            );
            return Err("Window Creation Failed!".into());
        }

        Ok(window_handle)
    }
}

fn run_message_loop(window: HWND) -> WPARAM {
    unsafe {
        // Read up on this. https://doc.rust-lang.org/std/mem/union.MaybeUninit.html
        let mut msg: MSG = std::mem::uninitialized();

        loop {
            if GetMessageW(&mut msg, window, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            } else {
                return msg.wParam;
            }
        }
    }
}

// Encodes to a wide string
fn to_wstring(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}


fn main() {
    let window = create_main_window("Handmade Hero", "Handmade Hero").expect("Window creation failed");

    // unsafe {
    //     ShowWindow(hWnd, nCmdShow)
    // }

    run_message_loop(window);
}
