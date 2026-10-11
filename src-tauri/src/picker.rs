use device_query::{DeviceQuery, DeviceState};
use serde::Serialize;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_clipboard_manager::ClipboardExt;
use xcap::Monitor;

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

#[derive(Default)]
pub struct PickerState {
    opening: AtomicBool,
    ready: AtomicBool,
    running: AtomicBool,
    radius: AtomicU32,
    frame: Mutex<Option<Frame>>,
}

#[derive(Clone, Serialize)]
pub struct Frame {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    hex: String,
    sequence: u64,
}

struct OpeningGuard<'a>(&'a AtomicBool);
impl Drop for OpeningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn deliver_color(app: &tauri::AppHandle, hex: String) -> Result<(), String> {
    app.clipboard()
        .write_text(&hex)
        .map_err(|e| e.to_string())?;
    app.emit_to("main", "color-picked", hex)
        .map_err(|e| e.to_string())
}

// WebView2 creation must not run inside synchronous commands or event handlers.
#[tauri::command]
pub async fn open_color_picker(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<PickerState>();
    if state.opening.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let _guard = OpeningGuard(&state.opening);
    if app.get_webview_window("picker").is_some() {
        return Ok(());
    }
    if state.running.load(Ordering::SeqCst) {
        return Err("The previous picker is closing. Please try again.".into());
    }

    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
    {
        // The compositor owns live picking on Wayland, via the PickColor portal.
        match ashpd::desktop::Color::pick().send().await {
            Ok(request) => match request.response() {
                Ok(color) => {
                    let channel = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    deliver_color(
                        &app,
                        format!(
                            "#{:02X}{:02X}{:02X}",
                            channel(color.red()),
                            channel(color.green()),
                            channel(color.blue())
                        ),
                    )?;
                }
                Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {}
                Err(error) => return Err(format!("Wayland color picker: {error}")),
            },
            Err(error) => {
                return Err(format!(
                    "The desktop's PickColor portal is unavailable: {error}"
                ))
            }
        }
        return Ok(());
    }

    // Validate capture permission before opening a topmost window.
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        if !unsafe { CGPreflightScreenCaptureAccess() } {
            return Err("Allow ProColorator in System Settings → Privacy & Security → Screen Recording, then restart the app".to_string());
        }
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        let monitor = monitors.first().ok_or("No displays available")?;
        monitor
            .capture_region(0, 0, 1, 1)
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| e.to_string())??;

    state.ready.store(false, Ordering::SeqCst);
    state.radius.store(30, Ordering::SeqCst);
    *state.frame.lock().map_err(|e| e.to_string())? = None;
    let builder = WebviewWindowBuilder::new(&app, "picker", WebviewUrl::App("picker".into()))
        .title("ProColorator Picker — Esc to cancel")
        .inner_size(240.0, 260.0)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false);
    #[cfg(target_os = "windows")]
    let builder = builder.content_protected(true);
    let picker = builder.build().map_err(|e| e.to_string())?;
    let watchdog_app = app.clone();
    std::thread::spawn(move || {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(8) {
            if watchdog_app
                .state::<PickerState>()
                .ready
                .load(Ordering::SeqCst)
            {
                return;
            }
            if picker.inner_size().is_err() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = picker.close();
        let _ = watchdog_app.emit_to(
            "main",
            "picker-error",
            "Picker did not load. Please try again.",
        );
    });
    Ok(())
}

#[tauri::command]
pub async fn picker_ready(app: tauri::AppHandle) -> Result<(), String> {
    let picker = app.get_webview_window("picker").ok_or("Picker is closed")?;
    if app
        .state::<PickerState>()
        .ready
        .swap(true, Ordering::SeqCst)
    {
        return Ok(());
    }
    picker
        .set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    app.state::<PickerState>()
        .running
        .store(true, Ordering::SeqCst);
    std::thread::spawn(move || {
        if let Err(error) = live_loop(&app, &picker) {
            let _ = app.emit_to("main", "picker-error", error);
        }
        let _ = picker.close();
        if let Some(main) = app.get_webview_window("main") {
            let _ = main.set_focus();
        }
        app.state::<PickerState>()
            .running
            .store(false, Ordering::SeqCst);
    });
    Ok(())
}

fn live_loop(app: &tauri::AppHandle, picker: &tauri::WebviewWindow) -> Result<(), String> {
    let device = DeviceState::checked_new()
        .ok_or("Allow Accessibility access on macOS, or check the X11 display connection")?;
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let input = crate::picker_input::Guard::start(device.get_mouse().coords);
    #[cfg(target_os = "linux")]
    let input = crate::picker_input::Guard::start()?;
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let mut sequence = 0;
    while picker.inner_size().is_ok() {
        let mouse = device.get_mouse();
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        let (selected, cancel, zoom) = {
            if let Some(error) = input.0.error.lock().map_err(|e| e.to_string())?.as_ref() {
                return Err(error.clone());
            }
            let zoom = input.0.zoom.swap(0, Ordering::SeqCst);
            let selected = if input.0.confirm.swap(false, Ordering::SeqCst) {
                Some(
                    input
                        .0
                        .selected_position
                        .lock()
                        .map_err(|e| e.to_string())?
                        .take()
                        .unwrap_or(mouse.coords),
                )
            } else {
                None
            };
            (selected, input.0.cancel.load(Ordering::SeqCst), zoom)
        };
        #[cfg(target_os = "linux")]
        let (selected, cancel, zoom) = input.poll();
        if cancel {
            return Ok(());
        }
        if zoom != 0 {
            let state = app.state::<PickerState>();
            state.radius.store(
                (state.radius.load(Ordering::SeqCst) as i32 + zoom * 4).clamp(8, 100) as u32,
                Ordering::SeqCst,
            );
        }
        let radius = app
            .state::<PickerState>()
            .radius
            .load(Ordering::SeqCst)
            .clamp(8, 100);
        let (x, y) = selected.unwrap_or(mouse.coords);
        for monitor in &monitors {
            let mx = monitor.x().map_err(|e| e.to_string())?;
            let my = monitor.y().map_err(|e| e.to_string())?;
            let mw = monitor.width().map_err(|e| e.to_string())?;
            let mh = monitor.height().map_err(|e| e.to_string())?;
            if x < mx || y < my || x >= mx + mw as i32 || y >= my + mh as i32 {
                continue;
            }
            let px = (x - mx) as u32;
            let py = (y - my) as u32;
            let sx = px.saturating_sub(radius);
            let sy = py.saturating_sub(radius);
            let right = (px + radius + 1).min(mw);
            let bottom = (py + radius + 1).min(mh);
            #[cfg(not(target_os = "macos"))]
            let scale = monitor.scale_factor().map_err(|e| e.to_string())? as f64;
            #[cfg(target_os = "macos")]
            let scale = 1.0;
            let panel_w = (240.0 * scale).ceil() as i32;
            let panel_h = (260.0 * scale).ceil() as i32;
            let gap = radius as i32 + 24;
            let wx = if x + gap + panel_w <= mx + mw as i32 {
                x + gap
            } else {
                x - gap - panel_w
            };
            let wy = if y + gap + panel_h <= my + mh as i32 {
                y + gap
            } else {
                y - gap - panel_h
            };
            #[cfg(not(target_os = "macos"))]
            picker
                .set_position(PhysicalPosition::new(wx, wy))
                .map_err(|e| e.to_string())?;
            #[cfg(target_os = "macos")]
            picker
                .set_position(tauri::LogicalPosition::new(wx as f64, wy as f64))
                .map_err(|e| e.to_string())?;
            // Let the compositor move the previous panel away before capturing.
            std::thread::sleep(Duration::from_millis(16));
            let crop = monitor
                .capture_region(sx, sy, right - sx, bottom - sy)
                .map_err(|e| {
                    format!("Screen capture failed. Check screen recording permissions: {e}")
                })?;
            // macOS captures backing pixels for a rectangle expressed in points.
            let capture_scale_x = crop.width() as f64 / (right - sx) as f64;
            let capture_scale_y = crop.height() as f64 / (bottom - sy) as f64;
            let cx = (((px - sx) as f64 * capture_scale_x).floor() as u32).min(crop.width() - 1);
            let cy = (((py - sy) as f64 * capture_scale_y).floor() as u32).min(crop.height() - 1);
            let pixel = crop.get_pixel(cx, cy);
            let hex = format!("#{:02X}{:02X}{:02X}", pixel[0], pixel[1], pixel[2]);
            if selected.is_some() {
                return deliver_color(app, hex);
            }
            // Pad clipped edges so the crosshair still marks the sampled pixel.
            let side = radius * 2 + 1;
            let crop = image::imageops::resize(
                &crop,
                right - sx,
                bottom - sy,
                image::imageops::FilterType::Nearest,
            );
            let mut padded = image::RgbaImage::new(side, side);
            image::imageops::replace(
                &mut padded,
                &crop,
                (radius - (px - sx)) as i64,
                (radius - (py - sy)) as i64,
            );
            sequence += 1;
            *app.state::<PickerState>()
                .frame
                .lock()
                .map_err(|e| e.to_string())? = Some(Frame {
                rgba: padded.into_raw(),
                width: side,
                height: side,
                hex,
                sequence,
            });
            picker.show().map_err(|e| e.to_string())?;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

#[tauri::command]
pub fn get_picker_frame(state: tauri::State<PickerState>) -> Result<Option<Frame>, String> {
    Ok(state.frame.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub fn set_picker_radius(state: tauri::State<PickerState>, radius: u32) {
    state.radius.store(radius.clamp(8, 100), Ordering::SeqCst);
}

#[tauri::command]
pub async fn cancel_picking(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(picker) = app.get_webview_window("picker") {
        picker.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
