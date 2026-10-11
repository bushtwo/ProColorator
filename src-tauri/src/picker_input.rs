//! Capture selection input without activating the desktop app underneath.
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod native {
    use rdev::{Button, EventType, Key};
    use std::cell::RefCell;
    use std::sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc, Mutex, OnceLock,
    };

    #[derive(Default)]
    pub struct Input {
        pub active: AtomicBool,
        pub confirm: AtomicBool,
        pub cancel: AtomicBool,
        pub zoom: AtomicI32,
        pub position: Mutex<(i32, i32)>,
        pub selected_position: Mutex<Option<(i32, i32)>>,
        pub error: Mutex<Option<String>>,
    }
    pub fn shared() -> Arc<Input> {
        static INPUT: OnceLock<Arc<Input>> = OnceLock::new();
        INPUT
            .get_or_init(|| {
                let input = Arc::new(Input::default());
                let events = input.clone();
                std::thread::spawn(move || {
                    let callback_input = events.clone();
                    let consumed = RefCell::new((false, Vec::new()));
                    let result = rdev::grab(move |event| {
                        let mut consumed = consumed.borrow_mut();
                        let active = callback_input.active.load(Ordering::SeqCst);
                        match event.event_type {
                            EventType::ButtonPress(Button::Left) if active => {
                                consumed.0 = true;
                                *callback_input.selected_position.lock().unwrap() =
                                    Some(*callback_input.position.lock().unwrap());
                                callback_input.confirm.store(true, Ordering::SeqCst);
                                return None;
                            }
                            EventType::ButtonRelease(Button::Left) if consumed.0 => {
                                consumed.0 = false;
                                return None;
                            }
                            EventType::KeyPress(key) if active => {
                                let handled = match key {
                                    Key::Escape => {
                                        callback_input.cancel.store(true, Ordering::SeqCst);
                                        true
                                    }
                                    Key::Return => {
                                        callback_input.confirm.store(true, Ordering::SeqCst);
                                        true
                                    }
                                    Key::Equal => {
                                        callback_input.zoom.fetch_sub(1, Ordering::SeqCst);
                                        true
                                    }
                                    Key::Minus => {
                                        callback_input.zoom.fetch_add(1, Ordering::SeqCst);
                                        true
                                    }
                                    _ => false,
                                };
                                if handled {
                                    if !consumed.1.contains(&key) {
                                        consumed.1.push(key);
                                    }
                                    return None;
                                }
                            }
                            EventType::KeyRelease(key) if consumed.1.contains(&key) => {
                                consumed.1.retain(|held| *held != key);
                                return None;
                            }
                            EventType::Wheel { delta_y, .. } if active => {
                                callback_input
                                    .zoom
                                    .fetch_add(-delta_y.signum() as i32, Ordering::SeqCst);
                                return None;
                            }
                            EventType::MouseMove { x, y } => {
                                if active {
                                    *callback_input.position.lock().unwrap() = (x as i32, y as i32);
                                }
                            }
                            _ => {}
                        }
                        Some(event)
                    });
                    if let Err(error) = result {
                        *events.error.lock().unwrap() =
                            Some(format!("Native input permission or hook failed: {error:?}"));
                    }
                });
                input
            })
            .clone()
    }
    pub struct Guard(pub Arc<Input>);
    impl Guard {
        pub fn start(position: (i32, i32)) -> Self {
            let input = shared();
            input.confirm.store(false, Ordering::SeqCst);
            input.cancel.store(false, Ordering::SeqCst);
            input.zoom.store(0, Ordering::SeqCst);
            *input.position.lock().unwrap() = position;
            *input.selected_position.lock().unwrap() = None;
            input.active.store(true, Ordering::SeqCst);
            Self(input)
        }
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.active.store(false, Ordering::SeqCst);
        }
    }
}
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use native::Guard;

#[cfg(target_os = "linux")]
pub struct Guard(*mut x11::xlib::Display);
#[cfg(target_os = "linux")]
impl Guard {
    pub fn start() -> Result<Self, String> {
        use x11::xlib::*;
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return Err("Cannot connect to the X11 display".into());
            }
            let root = XDefaultRootWindow(display);
            let pointer = XGrabPointer(
                display,
                root,
                0,
                (ButtonPressMask | ButtonReleaseMask | PointerMotionMask) as u32,
                GrabModeAsync,
                GrabModeAsync,
                0,
                0,
                CurrentTime,
            );
            if pointer != GrabSuccess {
                XCloseDisplay(display);
                return Err("Another application is already grabbing the pointer".into());
            }
            let keyboard =
                XGrabKeyboard(display, root, 0, GrabModeAsync, GrabModeAsync, CurrentTime);
            if keyboard != GrabSuccess {
                XUngrabPointer(display, CurrentTime);
                XCloseDisplay(display);
                return Err("Another application is already grabbing the keyboard".into());
            }
            XFlush(display);
            Ok(Self(display))
        }
    }

    pub fn poll(&self) -> (Option<(i32, i32)>, bool, i32) {
        use x11::xlib::*;
        let mut selected = None;
        let mut cancel = false;
        let mut zoom = 0;
        unsafe {
            while XPending(self.0) > 0 {
                let mut event: XEvent = std::mem::zeroed();
                XNextEvent(self.0, &mut event);
                match event.get_type() {
                    ButtonPress => match event.button.button {
                        1 => selected = Some((event.button.x_root, event.button.y_root)),
                        4 => zoom -= 1,
                        5 => zoom += 1,
                        _ => {}
                    },
                    KeyPress => match XLookupKeysym(&mut event.key, 0) as u32 {
                        x11::keysym::XK_Escape => cancel = true,
                        x11::keysym::XK_Return => {
                            selected = Some((event.key.x_root, event.key.y_root))
                        }
                        x11::keysym::XK_equal => zoom -= 1,
                        x11::keysym::XK_minus => zoom += 1,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        (selected, cancel, zoom)
    }
}
#[cfg(target_os = "linux")]
impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            x11::xlib::XUngrabPointer(self.0, x11::xlib::CurrentTime);
            x11::xlib::XUngrabKeyboard(self.0, x11::xlib::CurrentTime);
            x11::xlib::XCloseDisplay(self.0);
        }
    }
}
