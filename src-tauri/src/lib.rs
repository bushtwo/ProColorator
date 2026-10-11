mod picker;
mod picker_input;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(picker::PickerState::default())
        .setup(|app| {
            if let Err(error) = app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_shortcuts(["ctrl+shift+c"])?
                    .with_handler(|app, _, event| {
                        if event.state == ShortcutState::Pressed {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(error) = picker::open_color_picker(app.clone()).await {
                                    let _ = app.emit_to("main", "picker-error", error);
                                }
                            });
                        }
                    })
                    .build(),
            ) {
                // A reserved or unsupported shortcut must not prevent app startup.
                eprintln!("Global shortcut unavailable: {error}");
            }
            if let Some(main) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                main.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Destroyed) {
                        if let Some(picker) = handle.get_webview_window("picker") {
                            let _ = picker.close();
                        }
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            picker::open_color_picker,
            picker::picker_ready,
            picker::get_picker_frame,
            picker::set_picker_radius,
            picker::cancel_picking
        ])
        .run(tauri::generate_context!())
        .expect("error while running ProColorator");
}
