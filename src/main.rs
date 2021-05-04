
// #[cfg(windows)] extern crate winapi;

#![cfg(windows)]
// Let's put this so that it won't open the console
#![windows_subsystem = "windows"]

use std::{error::Error, mem};
use std::ptr::null_mut;
use winapi::{shared::minwindef::*, um::wingdi::{BLACKNESS, DIB_RGB_COLORS, PatBlt, SRCCOPY}};
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
                rcPaint: RECT { left: 0, right: 100, top: 0, bottom: 100 }, // TODO - need to think about what values are actually good here.
                fRestore: 0, 
                fIncUpdate: 0, 
                rgbReserved: [0; 32],
            };

            let device_context: HDC = BeginPaint(window, &mut paint);
            let x = paint.rcPaint.left;
            let y = paint.rcPaint.top;
            let height = paint.rcPaint.bottom - paint.rcPaint.top;
            let width = paint.rcPaint.right - paint.rcPaint.left;

            let window_dimension = get_window_dimension(window);

            // TODO - create a global buffer... lazy static?
            // Looks like can't pass in buffer via this callback.
            blit_buffer_to_window(&offscreen_buffer, device_context, window_dimension.width, window_dimension.height);

            // PatBlt(device_context, x, y, width, height, BLACKNESS); // No Longer Needed
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
            return Err("Window Creation Failed!".into());
        }

        Ok(window_handle)
    }
}

pub fn blit_buffer_to_window(buffer: &GameOffScreenBuffer, device_context: HDC, window_width: i32, window_height: i32) {
    unsafe {
        winapi::um::wingdi::StretchDIBits(
            device_context,
            0, 0, window_width, window_height,
            0, 0, buffer.width, buffer.height,
            buffer.memory,
            buffer_info_pointer,
            DIB_RGB_COLORS,
            SRCCOPY
        );
    }
}

fn run_message_loop(window: HWND) -> WPARAM {
    let x_offset = 0;
    let y_offset = 0;

    unsafe {
        // Read up on this. https://doc.rust-lang.org/std/mem/union.MaybeUninit.html
        let mut msg: MSG = std::mem::uninitialized();

        loop {
            let offscreen_buffer = GameOffScreenBuffer {
                // memory = ,
                // width = ,
                // height = ,
                // pitch = ,
            };

            // update and write to game buffer here.

            let device_context = GetDC(window);
            let window_dimension = get_window_dimension(window);
            blit_buffer_to_window(&offscreen_buffer, device_context, window_dimension.width, window_dimension.height);


            ReleaseDC(window, device_context);





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

pub struct GameOffScreenBuffer {
    
    pub memory: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
}

pub struct WindowDimension {
    pub width: i32,
    pub height: i32,
}

pub fn get_window_dimension(window: HWND) -> WindowDimension {
    let width = 0;
    let height = 0;

    unsafe {
        let client_rect: RECT = std::mem::uninitialized();
        GetClientRect(window, client_rect);
        width = client_rect.right - client_rect.left;
        height = client_rect.bottom - client_rect.top;
    };

    WindowDimension {
        width,
        height,
    }
}

pub fn resize_device_independent_buffer_section(buffer: &GameOffScreenBuffer, width: u32, height: u32) {
    // Clear Buffer memory? There is a VirtualFree here in the C code.

    buffer.width = width;
    buffer.height = height;
    let bytes_per_pixel = 4;

    // Create bmiHeader which ends up being the buffer_info_pointer field.

    let bitmap_memory_size = buffer.width * buffer.height * bytes_per_pixel;
    buffer.pitch = width * bytes_per_pixel;

    // VirtualAlloc the buffer here with 0, memory size, MEM_COMMIT, PAGE_READWRITE
}

pub fn render_weird_gradient(buffer: &GameOffScreenBuffer, x_offset: i32, y_offset: i32) {
    // game buffer is void* in C. Cast to u8 for pointer arithmetic?

    for y in 0..buffer.height {
        // create a u32 pixel here. 
        // xx RR GG BB (Little Endian) 
        // 00 00 00 00 
        for x in 0..buffer.width {
            let blue = x + x_offset as u32;
            let green = y + y_offset as u32;

            let four_byte_pixel: u32 = (green << 8) | blue;
        }

        // Add the pitch to the row here, because the pitch is how many bytes make up a row of pixels.
    }
}
