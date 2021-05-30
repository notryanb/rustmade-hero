

#![cfg(windows)]
// Let's put this so that it won't open the console
#![windows_subsystem = "windows"]

// use std::sync::{Arc, Mutex};
use std::{error::Error};
use std::ptr::null_mut;
use winapi::{
    ctypes::c_void,
    shared::minwindef::*,
    um::wingdi::{
        BITMAPINFO, 
        BITMAPINFOHEADER, 
        BI_RGB, 
        DIB_RGB_COLORS, 
        SRCCOPY
    }
};
use winapi::shared::windef::*;
use winapi::um::libloaderapi::{GetModuleHandleW};
use winapi::um::winuser::*;

const BUFFER_SIZE: usize = 1280 * 720 * 4;

pub struct GameOffScreenBuffer {
    pub memory: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
}

pub struct WindowDimension {
    pub width: i32,
    pub height: i32,
}

trait WindowStuff {
    fn window_proc(&self, window: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT>;
}

impl WindowStuff for GameOffScreenBuffer {
    // Window procedure function to handle events
    fn window_proc(
        &self,
        window: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
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
    
                let device_context: Option<HDC> = begin_paint(window, &mut paint);//  unsafe { BeginPaint(window, &mut paint); }
                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;
                let height = paint.rcPaint.bottom - paint.rcPaint.top;
                let width = paint.rcPaint.right - paint.rcPaint.left;
    
                let window_dimension = get_window_dimension(window);

                if device_context.is_some() {

                    blit_buffer_to_window(&self, device_context.unwrap(), window_dimension.width, window_dimension.height);
        
                    // PatBlt(device_context, x, y, width, height, BLACKNESS); // No Longer Needed
                    end_paint(window, &paint);
                }    
            }
            _ => return Some(def_window_proc_w(window, msg, wparam, lparam)),
        }

        return None;
    }
}

fn def_window_proc_w(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {    DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn begin_paint(hwnd: HWND, paint: &mut PAINTSTRUCT) -> Option<HDC> {
    let device_context =  unsafe { BeginPaint(hwnd, paint) };

    Some(device_context)
}

pub fn end_paint(hwnd: HWND, paint: &PAINTSTRUCT) {
    unsafe { EndPaint(hwnd, paint); }
}




fn create_main_window(name: &str, title: &str, offscreen_buffer: &mut GameOffScreenBuffer) -> Result<HWND, Box<dyn Error>> {
    let name = to_wstring(name);
    let title = to_wstring(title);

    unsafe {
        let instance = GetModuleHandleW(null_mut());

        let window_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,

            // TODO - Figure out how to get this pointer....
            lpfnWndProc: Some(&offscreen_buffer.window_proc),
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
            null_mut()
        );
        

        if window_handle.is_null() {
            return Err("Window Creation Failed!".into());
        }

        Ok(window_handle)
    }
}

pub fn blit_buffer_to_window(buffer: &GameOffScreenBuffer, device_context: HDC, window_width: i32, window_height: i32) {

    let bitmap_info: BITMAPINFO = unsafe { std::mem::zeroed::<BITMAPINFO>() };
    let mut bmi_header: BITMAPINFOHEADER = unsafe { std::mem::zeroed::<BITMAPINFOHEADER>() };
    bmi_header.biWidth = buffer.width;
    bmi_header.biHeight = buffer.height;
    bmi_header.biPlanes = 1;
    bmi_header.biBitCount = 32;
    bmi_header.biCompression = BI_RGB;
    bmi_header.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32; // usize to u32... unsure about this.

    let buffer_memory: *const c_void = &buffer.memory as *const _ as *const c_void;

    unsafe {
        winapi::um::wingdi::StretchDIBits(
            device_context,
            0, 0, window_width, window_height,
            0, 0, buffer.width, buffer.height,
            buffer_memory,
            &bitmap_info,
            DIB_RGB_COLORS,
            SRCCOPY
        );
    }
}

fn run_message_loop(offscreen_buffer: &mut GameOffScreenBuffer, window: HWND) -> WPARAM {
    let x_offset = 0;
    let y_offset = 0;
    let mut msg: MSG = unsafe { std::mem::zeroed() };



    loop {    
        unsafe {
            // update and write to game buffer here.

            render_weird_gradient(&mut offscreen_buffer, x_offset, y_offset);

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
    
    let mut offscreen_buffer = GameOffScreenBuffer {
        memory: Vec::with_capacity(1280 * 720 * 4),
        width: 1280,
        height: 720,
        pitch: 4,
    };

    let window = create_main_window("Handmade Hero", "Handmade Hero", &mut offscreen_buffer).expect("Window creation failed");

    // unsafe {
    //     ShowWindow(hWnd, nCmdShow)
    // }

    run_message_loop(&mut offscreen_buffer, window);
}

pub fn get_window_dimension(window: HWND) -> WindowDimension {
    let mut client_rect: RECT = unsafe { std::mem::zeroed::<RECT>() };
    unsafe { GetClientRect(window, &mut client_rect); }
    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    WindowDimension {
        width,
        height,
    }
}

// pub fn resize_device_independent_buffer_section(buffer: &GameOffScreenBuffer, width: i32, height: i32) {
//     // Clear Buffer memory? There is a VirtualFree here in the C code.

//     buffer.width = width;
//     buffer.height = height;
//     let bytes_per_pixel = 4;

//     // Create bmiHeader which ends up being the buffer_info_pointer field.

//     let bitmap_memory_size = buffer.width * buffer.height * bytes_per_pixel;
//     buffer.pitch = width * bytes_per_pixel;

//     // VirtualAlloc the buffer here with 0, memory size, MEM_COMMIT, PAGE_READWRITE
// }

pub fn render_weird_gradient(buffer: &mut GameOffScreenBuffer, x_offset: i32, y_offset: i32) {
    // game buffer is void* in C. Cast to u8 for pointer arithmetic?


    for y in 0..buffer.height {
        // create a u32 pixel here. 
        // xx RR GG BB (Little Endian) 
        // 00 00 00 00 
        for x in 0..buffer.width {
            let blue = x + x_offset;
            let green = y + y_offset;

            let pixel_index  = (y + x * buffer.height) as usize;

            buffer.memory[pixel_index] = (blue / 255) as u8;
            buffer.memory[pixel_index + 1] = (green / 255) as u8;

            
            // let blue = x + x_offset as u32;
            // let green = y + y_offset as u32;

            // let four_byte_pixel: u32 = (green << 8) | blue;
        }

        // Add the pitch to the row here, because the pitch is how many bytes make up a row of pixels.
    }
}
