#![cfg(windows)]

// Let's put this so that it won't open the console
// NOTE - using this will stop console logging
// #![windows_subsystem = "windows"]

extern crate pretty_env_logger;
#[macro_use]
extern crate log;

use std::any::Any;
use std::error::Error;
use std::ptr::null_mut;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::*;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::*;
use winapi::{
    shared::minwindef::*,
    um::wingdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY},
};



pub trait WinHandler {
    // fn connect(&mut self, handle: &WindowHandle);

    fn as_any(&mut self) -> &mut dyn Any;
}


struct WindowState {
    hwnd: Cell<HWND>,
    wndproc: Box<dyn WindowProc>,
}


struct WndState {
    game_offscreen_buffer: GameOffScreenBuffer,
    handler: Box<dyn WinHandler>,
    // render_target: Option<DeviceContext>,
}

struct MyWndProc {
    handle: RefCell<WindowHandle>,
    state: RefCell<Option<WndState>>,
}

impl MyWndProc {
    fn with_wnd_state<F, R>(&self, f: F) -> Option<R> 
        where F: FnOnce(&mut WndState) -> R,
    {
        // info!("with wnd stats");

        let ret = if let Ok(mut s) = self.state.try_borrow_mut() {
            (*s).as_mut().map(|state| f(state))
        } else {
            error!("failed to borrow WndState at");
            None
        };
        if ret.is_some() {
            // info!("ret is some");
            // self.handle_deferred_queue();
        }
        ret
    }
}



#[derive(Clone)]
pub struct WindowHandle {
    state: Weak<WindowState>,
}

pub struct GameOffScreenBuffer {
    pub memory: Vec<u8>,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub bitmap_info: BITMAPINFO,
}

pub struct WindowDimension {
    pub width: i32,
    pub height: i32,
}

trait WindowProc {
    fn connect(&self, handle: &WindowHandle, state: WndState);

    fn cleanup(&self, hwnd: HWND);

    fn window_proc(
        &self,
        window: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT>;
}

impl WindowProc for MyWndProc {
    fn connect(&self, handle: &WindowHandle, state: WndState) {
        *self.handle.borrow_mut() = handle.clone();
        *self.state.borrow_mut() = Some(state);
        // self.state
        //     .borrow_mut()
        //     .as_mut()
        //     .unwrap()
        //     .handler
        //     .scale(self.scale());
    }

    fn cleanup(&self, hwnd: HWND) {
        // self.app.remove_window(hwnd);
    }

    fn window_proc(
        &self,
        window: HWND,
        msg: UINT,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
        info!("Inside window_proc");

        match msg {
            WM_PAINT => {
                info!("WM_PAINT called Safe");

                let mut paint: PAINTSTRUCT = unsafe { std::mem::zeroed() };
                let device_context: Option<HDC> = begin_paint(window, &mut paint);
                let window_dimension = get_window_dimension(window);

                if device_context.is_some() {
                    self.with_wnd_state(|s| {
                        render_weird_gradient(&mut s.game_offscreen_buffer, 0, 0);

                        blit_buffer_to_window(
                            &s.game_offscreen_buffer,
                            device_context.unwrap(),
                            window_dimension.width,
                            window_dimension.height,
                        );
                        end_paint(window, &paint);
                        Some(0);
                    });
                    
                } else {
                    warn!("No device context");
                }
            }
            _ => return Some(def_window_proc_w(window, msg, wparam, lparam)),
        }

        return None;
    }
}

fn def_window_proc_w(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub fn begin_paint(hwnd: HWND, paint: &mut PAINTSTRUCT) -> Option<HDC> {
    let device_context = unsafe { BeginPaint(hwnd, paint) };

    Some(device_context)
}

pub fn end_paint(hwnd: HWND, paint: &PAINTSTRUCT) {
    unsafe {
        EndPaint(hwnd, paint);
    }
}


#[cfg(target_arch = "x86_64")]
type WindowLongPtr = winapi::shared::basetsd::LONG_PTR;
#[cfg(target_arch = "x86")]
type WindowLongPtr = LONG;


pub(crate) unsafe extern "system" fn win_proc_proxy(
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CREATE {
        let create_struct = &*(lparam as *const CREATESTRUCTW);
        let wndproc_ptr = create_struct.lpCreateParams;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, wndproc_ptr as WindowLongPtr);
    }

    let window_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
    let result = {
        if window_ptr.is_null() {
            None
        } else {
            (*window_ptr).wndproc.window_proc(hwnd, msg, wparam, lparam)
        }
    };

    // if msg == WM_NCDESTROY && !window_ptr.is_null() {
    //     (*window_ptr).cleanup(hwnd);
    //     SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    //     std::mem::drop(std::rc::Rc::from_raw(window_ptr));
    // }

    match result {
        Some(lresult) => lresult,
        None => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}


// fn create_main_window(name: &str, title: &str) -> Result<HWND, Box<dyn Error>> {
//     let name = to_wstring(name);
//     let title = to_wstring(title);

//     unsafe {
//         let instance = GetModuleHandleW(null_mut());

//         let window_class = WNDCLASSW {
//             // cbSize: std::mem::size_of::<WNDCLASSW>() as u32,
//             // style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
//             style: CS_CLASSDC | CS_HREDRAW | CS_VREDRAW,
//             lpfnWndProc: Some(win_proc_proxy),
//             cbClsExtra: 0,
//             cbWndExtra: 0,
//             hInstance: 0 as HINSTANCE,
//             hIcon: LoadIconW(0 as HINSTANCE, IDI_APPLICATION),
//             hCursor: LoadCursorW(0 as HINSTANCE, IDC_ARROW),
//             hbrBackground: COLOR_WINDOWFRAME as HBRUSH,
//             lpszMenuName: 0 as LPCWSTR,
//             lpszClassName: name.as_ptr(),
//             // hIconSm: LoadIconW(null_mut(), IDI_APPLICATION),
//         };

//         if RegisterClassW(&window_class) == 0 {
//             // Probably do without messaging and use Rust tracing/logging
//             // unless this needs to be shown to a user
//             MessageBoxW(
//                 null_mut(),
//                 to_wstring("Window Registration Failed!").as_ptr(),
//                 to_wstring("Error").as_ptr(),
//                 MB_ICONEXCLAMATION | MB_OK,
//             );
//             return Err("Window Registration Failed".into());
//         }

//         let window_handle = CreateWindowExW(
//             0,
//             name.as_ptr(),
//             title.as_ptr(),
//             WS_OVERLAPPEDWINDOW | WS_VISIBLE,
//             CW_USEDEFAULT,
//             CW_USEDEFAULT,
//             CW_USEDEFAULT,
//             CW_USEDEFAULT,
//             null_mut(),
//             null_mut(),
//             instance,
//             null_mut(),
//         );

//         if window_handle.is_null() {
//             return Err("Window Creation Failed!".into());
//         }

//         Ok(window_handle)
//     }
// }

pub fn blit_buffer_to_window(
    buffer: &GameOffScreenBuffer,
    device_context: HDC,
    window_width: i32,
    window_height: i32,
) {
    
    let buffer_memory = buffer.memory.as_ptr() as *mut core::ffi::c_void;

    unsafe {
        winapi::um::wingdi::StretchDIBits(
            device_context,
            0,
            0,
            window_width,
            window_height,
            0,
            0,
            buffer.width,
            buffer.height,
            buffer_memory,
            &buffer.bitmap_info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

// TODO - don't really need to do anything with WPARAM - change return type?
fn run_message_loop(handle: WindowHandle) {
    // let x_offset = 0;
    // let y_offset = 0;

    let mut msg: MSG = unsafe { std::mem::zeroed() };

    // info!("About to enter loop");

    let state = handle.state.upgrade().unwrap();
    let hwnd = state.hwnd.get();

    
    'gameloop: loop {
        // render_weird_gradient(offscreen_buffer, x_offset, y_offset);

        let device_context = unsafe { GetDC(hwnd) };
        // let window_dimension = get_window_dimension(hwnd);

        // blit_buffer_to_window(
        //     &offscreen_buffer,
        //     device_context,
        //     window_dimension.width,
        //     window_dimension.height,
        // );

        unsafe {
            ReleaseDC(hwnd, device_context);

            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) > 0 {
                // info!("Received message {:?}", &msg.message);

                if msg.message == WM_QUIT {
                    info!("Received quit message");
                    break 'gameloop;
                }

                TranslateMessage(&msg);
                DispatchMessageA(&msg);
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

impl Default for WindowHandle {
    fn default() -> Self {
        WindowHandle {
            state: Default::default(),
        }
    }
}

pub struct GameState;

impl WinHandler for GameState {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

fn main() {
    pretty_env_logger::init();
    info!("Booting app");

    let width = 1280;
    let height = 720;

    let bytes_per_pixel = 4;
    let buffer_size = width * height * bytes_per_pixel;

    let mut bitmap_info: BITMAPINFO = unsafe { std::mem::zeroed::<BITMAPINFO>() };
    let mut bmi_header: BITMAPINFOHEADER = unsafe { std::mem::zeroed::<BITMAPINFOHEADER>() };
    bmi_header.biWidth = width;
    bmi_header.biHeight = -height;
    bmi_header.biPlanes = 1;
    bmi_header.biBitCount = 32;
    bmi_header.biCompression = BI_RGB;
    bmi_header.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32; // usize to u32... unsure about this.
    bitmap_info.bmiHeader = bmi_header;

    let mut offscreen_buffer = GameOffScreenBuffer {
        memory: Vec::with_capacity(buffer_size as usize),
        width: 1280,
        height: 720,
        pitch: bytes_per_pixel as i32,
        bitmap_info: bitmap_info,
    };

    // Initialize the screen buffer to black
    for _ in 0..buffer_size {
        offscreen_buffer.memory.push(0);
    }

    let handler = Box::new(GameState);

    let wndproc = MyWndProc {
        handle: Default::default(),
        state: RefCell::new(None),
    };

    let window = WindowState {
        hwnd: Cell::new(0 as HWND),
        wndproc: Box::new(wndproc),
    };

    let win = Rc::new(window);

    let handle = WindowHandle {
        state: Rc::downgrade(&win),
    };

    let state = WndState {
        game_offscreen_buffer: offscreen_buffer,
        handler: handler,
    };

    win.wndproc.connect(&handle, state);

    // info!("buffer.len(), {:?}", &offscreen_buffer.memory.len());
    let name = to_wstring("Handmade Hero");
    let title = to_wstring("Handmade Hero");
    
    unsafe{
        let window_class = WNDCLASSW {
            // cbSize: std::mem::size_of::<WNDCLASSW>() as u32,
            // style: CS_OWNDC | CS_HREDRAW | CS_VREDRAW,
            style: CS_CLASSDC | CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(win_proc_proxy),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: 0 as HINSTANCE,
            hIcon: LoadIconW(0 as HINSTANCE, IDI_APPLICATION),
            hCursor: LoadCursorW(0 as HINSTANCE, IDC_ARROW),
            hbrBackground: COLOR_WINDOWFRAME as HBRUSH,
            lpszMenuName: 0 as LPCWSTR,
            lpszClassName: name.as_ptr(),
            // hIconSm: LoadIconW(null_mut(), IDI_APPLICATION),
        };

        if RegisterClassW(&window_class) == 0 {
            // Probably do without messaging and use Rust tracing/logging
            // unless this needs to be shown to a user
            MessageBoxW(
                null_mut(),
                to_wstring("Window Registration Failed!").as_ptr(),
                to_wstring("Error").as_ptr(),
                MB_ICONEXCLAMATION | MB_OK,
            );
            panic!("Window registration failed");
        }



        let hwnd = CreateWindowExW(
            0,
            name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0 as HWND,
            0 as HMENU,
            0 as HINSTANCE,
            Rc::into_raw(win) as LPVOID,
        );

        if hwnd.is_null() {
            panic!("hwnd is null!");
        }
    }

    info!("Created main window");
    run_message_loop(handle);
}

pub fn get_window_dimension(window: HWND) -> WindowDimension {
    let mut client_rect: RECT = unsafe { std::mem::zeroed::<RECT>() };
    unsafe {
        GetClientRect(window, &mut client_rect);
    }

    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    WindowDimension { width, height }
}

// This is obviously wrong, but at least stuff is showing up.
pub fn render_weird_gradient(buffer: &mut GameOffScreenBuffer, x_offset: i32, y_offset: i32) {
    for y in 0..buffer.height {
        // create a u32 pixel here.
        // xx RR GG BB (Little Endian)
        // 00 00 00 00
        for x in 0..buffer.width {
            let blue = x + x_offset as i32;
            let green = y + y_offset as i32;

            let pixel_index = (x + y * buffer.width) as usize;
            let offset = pixel_index * buffer.pitch as usize;

            buffer.memory[offset + 0] = (blue % 255) as u8;
            buffer.memory[offset + 1] = (green % 255) as u8
        }
    }
}
