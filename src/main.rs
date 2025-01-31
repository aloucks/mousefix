#![windows_subsystem = "windows"]

use std::{
    mem, ptr,
    sync::Mutex,
    time::{Duration, Instant},
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use windows_sys::Win32::{
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG,
        WH_MOUSE_LL, WM_MBUTTONDOWN,
    },
};

static LAST_CLICK_TIME: Mutex<Option<Instant>> = Mutex::new(None);
const DEBOUNCE_THRESHOLD: Duration = Duration::from_millis(200);

unsafe extern "system" fn mouse_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code >= 0 && (w_param == WM_MBUTTONDOWN as usize) {
        let mut last_time = LAST_CLICK_TIME.lock().unwrap();
        let now = Instant::now();

        if let Some(previous_time) = *last_time {
            let duration = now.duration_since(previous_time);
            if duration < DEBOUNCE_THRESHOLD {
                eprintln!("duration: {:?}", duration);
                return 1; // Suppress duplicate click
            }
        }
        *last_time = Some(now);
    }

    CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param)
}

fn main() {
    unsafe {
        let thread_id = 0;
        let hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_proc),
            GetModuleHandleW(ptr::null()),
            thread_id,
        );
        if hook == ptr::null_mut() {
            eprintln!("Failed to install hook");
            std::process::exit(1);
        }

        let menu = Menu::new();
        let exit_item = MenuItem::new("Exit", true, None);
        menu.append(&exit_item).unwrap();

        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Middle-click Debouncer Running")
            .with_icon(load_icon())
            .build()
            .unwrap();

        let mut msg: MSG = mem::zeroed();

        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
            if let Ok(event) = MenuEvent::receiver().try_recv() {
                if exit_item.id() == event.id() {
                    break;
                }
            }
        }
    };
}

fn load_icon() -> tray_icon::Icon {
    let icon_rgba = include_bytes!("mouse.raw").to_vec();
    let icon_width = 64;
    let icon_height = 64;
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
