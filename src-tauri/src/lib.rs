// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::io::Cursor;
use std::sync::Mutex;

use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};
use xcap::Monitor;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Holds the single full-screen screenshot taken the instant the
// picker opens, so the frontend can fetch it once and do all
// magnifier zooming/color-sampling itself by reading pixels back
// out of that already-loaded image -- no live IPC screen capture
// per mouse-move, and critically, no dependency on the picker
// window's own transparency actually compositing correctly.
//
// That's not a minor detail: on X11 window managers with no
// compositor running (i3 by default has none), a "transparent"
// window just renders as solid black instead of see-through, and
// a live screen-capture approach would then capture that black
// overlay instead of the real desktop underneath it. Capturing
// once, up front, before the (now fully opaque) overlay window
// even exists, sidesteps that dependency entirely.
struct PickerBackground(Mutex<Option<Vec<u8>>>);

fn capture_primary_monitor_png() -> Result<Vec<u8>, String> {
    let monitors = Monitor::all().map_err(|error| error.to_string())?;

    let monitor = monitors
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| "No primary monitor found".to_string())?;

    let image = monitor.capture_image().map_err(|error| error.to_string())?;

    let mut png_bytes: Vec<u8> = Vec::new();

    image
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;

    Ok(png_bytes)
}

/// Fetched once by the picker page on mount. Returns the raw PNG
/// bytes captured by `open_color_picker` just before the overlay
/// window was created.
#[tauri::command]
fn get_picker_background(
    background: tauri::State<PickerBackground>,
) -> Result<Vec<u8>, String> {
    let guard = background
        .0
        .lock()
        .map_err(|_| "Background lock was poisoned".to_string())?;

    guard
        .clone()
        .ok_or_else(|| "No background has been captured yet".to_string())
}

/// Called by the picker window when the user clicks to confirm a
/// color. Hands the hex value back to the main window, brings it
/// to the front, and closes the picker.
#[tauri::command]
fn finish_picking(app: tauri::AppHandle, hex: String) -> Result<(), String> {
    app.emit("color-picked", hex)
        .map_err(|error| error.to_string())?;

    if let Some(main_window) = app.get_webview_window("main") {
        main_window.set_focus().map_err(|error| error.to_string())?;
    }

    if let Some(picker_window) = app.get_webview_window("picker") {
        picker_window.close().map_err(|error| error.to_string())?;
    }

    Ok(())
}

/// Called on Escape -- just closes the picker without emitting a
/// color, and returns focus to the main window.
#[tauri::command]
fn cancel_picking(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.set_focus();
    }

    if let Some(picker_window) = app.get_webview_window("picker") {
        picker_window.close().map_err(|error| error.to_string())?;
    }

    Ok(())
}

/// Captures the primary monitor, stores it for `get_picker_background`
/// to hand off, then creates the picker window (or just focuses it
/// if one is already open, so a double-trigger can't spawn two).
/// The window is a normal OPAQUE window sized to the primary
/// monitor -- no transparency, so no compositor dependency.
#[tauri::command]
fn open_color_picker(
    app: tauri::AppHandle,
    background: tauri::State<PickerBackground>,
) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("picker") {
        existing.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }

    let png_bytes = capture_primary_monitor_png()?;

    *background
        .0
        .lock()
        .map_err(|_| "Background lock was poisoned".to_string())? = Some(png_bytes);

    let monitor = app
        .primary_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No primary monitor found".to_string())?;

    let scale_factor = monitor.scale_factor();
    let position = monitor.position();
    let size = monitor.size();

    // WebviewWindowBuilder::position/inner_size take LOGICAL
    // pixels, but tauri::Monitor reports PHYSICAL ones -- same
    // class of bug we already hit once on the main window's resize
    // handling, so converting explicitly here rather than assuming.
    let logical_x = position.x as f64 / scale_factor;
    let logical_y = position.y as f64 / scale_factor;
    let logical_width = size.width as f64 / scale_factor;
    let logical_height = size.height as f64 / scale_factor;

    WebviewWindowBuilder::new(&app, "picker", WebviewUrl::App("picker".into()))
        .title("ProColorator Picker")
        .position(logical_x, logical_y)
        .inner_size(logical_width, logical_height)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(true)
        .build()
        .map_err(|error| error.to_string())?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(PickerBackground(Mutex::new(None)))
        .setup(|app| {
            #[cfg(desktop)]
            {
                let handle = app.handle().clone();

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_shortcuts(["ctrl+shift+c"])?
                        .with_handler(move |app_handle, shortcut, event| {
                            if event.state == ShortcutState::Pressed
                                && shortcut.matches(
                                    Modifiers::CONTROL | Modifiers::SHIFT,
                                    Code::KeyC,
                                )
                            {
                                let state = app_handle.state::<PickerBackground>();
                                let _ = open_color_picker(handle.clone(), state);
                            }
                        })
                        .build(),
                )?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_picker_background,
            finish_picking,
            cancel_picking,
            open_color_picker
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
