//! The global key source on Windows (a low-level keyboard hook) and X11
//! (XInput 2 raw key events): the bound key with its modifiers, seen over
//! any application but not taken from it, so a letter still types in the
//! chat and elsewhere while it also talks, as Discord does (issue #49, human
//! decision 2026-11-12). global-hotkey grabbed the key, and no application
//! (Twicecat's own window included) received it while it was bound. macOS
//! keeps global-hotkey's grab: watching keys there needs the Input
//! Monitoring permission.
//!
//! Keys are named as winit names them (`KeyQ`, `Digit5`, `ShiftLeft`), from
//! the system's scancode, which are the names Twicecat's window gives the
//! bindings. The matching ([`KeyWatch`]) is pure and tested everywhere.

use crate::binding::{Held, KeyBinding};

/// Which modifier a key is, by its winit name (`None`: not a modifier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Logo,
}

fn modifier(name: &str) -> Option<(Modifier, bool)> {
    Some(match name {
        "ControlLeft" => (Modifier::Ctrl, false),
        "ControlRight" => (Modifier::Ctrl, true),
        "AltLeft" => (Modifier::Alt, false),
        "AltRight" => (Modifier::Alt, true),
        "ShiftLeft" => (Modifier::Shift, false),
        "ShiftRight" => (Modifier::Shift, true),
        "SuperLeft" | "MetaLeft" => (Modifier::Logo, false),
        "SuperRight" | "MetaRight" => (Modifier::Logo, true),
        _ => return None,
    })
}

/// The bound key seen through every key event of the system: it presses
/// when it goes down with exactly the binding's modifiers, and lets go when
/// it goes up (whatever the modifiers then, as in Twicecat's window).
pub(crate) struct KeyWatch {
    binding: KeyBinding,
    /// Each modifier held, left and right apart.
    held: [[bool; 2]; 4],
    pressed: bool,
}

impl KeyWatch {
    pub(crate) fn new(binding: KeyBinding) -> Self {
        Self {
            binding,
            held: [[false; 2]; 4],
            pressed: false,
        }
    }

    fn modifiers(&self) -> Held {
        let any = |m: Modifier| self.held[m as usize].iter().any(|h| *h);
        Held {
            ctrl: any(Modifier::Ctrl),
            alt: any(Modifier::Alt),
            shift: any(Modifier::Shift),
            logo: any(Modifier::Logo),
        }
    }

    /// One key went down or up (`name` as winit names it). Returns the new
    /// transmit state when it changes. Repeats of a held key change nothing.
    pub(crate) fn key(&mut self, name: &str, down: bool) -> Option<bool> {
        if let Some((m, right)) = modifier(name) {
            self.held[m as usize][usize::from(right)] = down;
            return None;
        }
        if !self.binding.is_key(name) {
            return None;
        }
        if down && !self.pressed && self.binding.has_modifiers(self.modifiers()) {
            self.pressed = true;
            return Some(true);
        }
        if !down && self.pressed {
            self.pressed = false;
            return Some(false);
        }
        None
    }

    /// Sets which modifiers are held, `[ctrl, alt, shift, logo]`, each
    /// `[left, right]`, from the system's own key state. On Windows the hook
    /// sees nothing while the secure desktop is up (Win+L, Ctrl+Alt+Del, an
    /// elevation prompt), so a modifier let go there would stay held.
    #[cfg_attr(not(any(test, target_os = "windows")), allow(dead_code))]
    pub(crate) fn set_held(&mut self, held: [[bool; 2]; 4]) {
        self.held = held;
    }

    /// Whether the key is held now (to let go when the source stops).
    pub(crate) fn pressed(&self) -> bool {
        self.pressed
    }
}

/// A key's winit name from the system's scancode (Windows: the scancode,
/// `0xE0` prefix for extended keys in the high byte; X11: the keycode less
/// 8). `None` for a scancode winit does not know.
#[cfg(any(target_os = "windows", target_os = "linux"))]
fn key_name(scancode: u32) -> Option<String> {
    use winit::keyboard::PhysicalKey;
    use winit::platform::scancode::PhysicalKeyExtScancode as _;
    match PhysicalKey::from_scancode(scancode) {
        PhysicalKey::Code(code) => Some(format!("{code:?}")),
        PhysicalKey::Unidentified(_) => None,
    }
}

#[cfg(target_os = "windows")]
pub(crate) use windows::Listener;

#[cfg(target_os = "windows")]
mod windows {
    use std::cell::RefCell;
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::thread::JoinHandle;

    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL,
        VK_RMENU, VK_RSHIFT, VK_RWIN,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_UP, MSG,
        PM_NOREMOVE, PeekMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        WH_KEYBOARD_LL, WM_QUIT, WM_USER,
    };

    use super::KeyWatch;
    use crate::binding::KeyBinding;
    use crate::{PttEvent, Shared, Source};

    /// What the hook procedure matches and reports to, on the hook's thread
    /// (the procedure runs there, from its message loop).
    struct Watch {
        keys: KeyWatch,
        shared: Arc<Shared>,
    }

    thread_local! {
        static WATCH: RefCell<Option<Watch>> = const { RefCell::new(None) };
    }

    /// The hook's thread. Dropping it ends the thread's message loop.
    pub(crate) struct Listener {
        thread_id: u32,
        thread: Option<JoinHandle<()>>,
    }

    impl Listener {
        /// Hooks the keyboard on a thread of its own and reports `Ready`;
        /// `Err` with the reason when the hook is refused (nothing reported).
        pub(crate) fn start(key: KeyBinding, shared: Arc<Shared>) -> Result<Self, String> {
            let (started, thread_id) = mpsc::channel();
            let events = shared.clone();
            let thread = std::thread::Builder::new()
                .name("ptt-keys".into())
                .spawn(move || run(key, events, &started))
                .map_err(|e| e.to_string())?;
            match thread_id.recv() {
                Ok(Ok(thread_id)) => {
                    shared.notify(PttEvent::Ready {
                        source: Source::Hotkey,
                        trigger: key.to_string(),
                    });
                    Ok(Self {
                        thread_id,
                        thread: Some(thread),
                    })
                }
                Ok(Err(reason)) => {
                    let _ = thread.join();
                    Err(reason)
                }
                Err(_) => {
                    let _ = thread.join();
                    Err("the key thread ended".into())
                }
            }
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            // SAFETY: a plain call; the thread has a message queue (`run`
            // made it before sending its ID).
            unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn run(key: KeyBinding, shared: Arc<Shared>, started: &mpsc::Sender<Result<u32, String>>) {
        // SAFETY: MSG is plain data; all zeroes is a valid value.
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        // SAFETY: plain calls with valid arguments. Peeking makes the
        // thread's message queue, so `Drop`'s WM_QUIT reaches it. A
        // low-level hook may name the executable's own module.
        let hook = unsafe {
            PeekMessageW(
                &mut msg,
                std::ptr::null_mut(),
                WM_USER,
                WM_USER,
                PM_NOREMOVE,
            );
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(on_key),
                GetModuleHandleW(std::ptr::null()),
                0,
            )
        };
        if hook.is_null() {
            let _ = started.send(Err(std::io::Error::last_os_error().to_string()));
            return;
        }
        WATCH.with(|watch| {
            *watch.borrow_mut() = Some(Watch {
                keys: KeyWatch::new(key),
                shared: shared.clone(),
            });
        });
        // SAFETY: a plain call.
        let _ = started.send(Ok(unsafe { GetCurrentThreadId() }));
        // The hook procedure runs from here, until WM_QUIT (0) or an error
        // (-1).
        // SAFETY: `msg` is a valid MSG to write into.
        while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {}
        // SAFETY: the hook this thread set.
        unsafe { UnhookWindowsHookEx(hook) };
        let watch = WATCH.with(|watch| watch.borrow_mut().take());
        if watch.is_some_and(|w| w.keys.pressed()) {
            shared.report(Source::Hotkey, false);
        }
    }

    /// The modifiers held now, as Windows sees them (`KeyWatch::set_held`).
    fn modifiers_held() -> [[bool; 2]; 4] {
        // SAFETY: a plain call; the high bit is set while the key is down.
        let down = |vk: VIRTUAL_KEY| unsafe { GetAsyncKeyState(i32::from(vk)) } < 0;
        [
            [down(VK_LCONTROL), down(VK_RCONTROL)],
            [down(VK_LMENU), down(VK_RMENU)],
            [down(VK_LSHIFT), down(VK_RSHIFT)],
            [down(VK_LWIN), down(VK_RWIN)],
        ]
    }

    /// The low-level keyboard hook procedure. Short: Windows drops a hook
    /// that keeps the keyboard waiting. The key always goes on to the
    /// application that has the focus.
    unsafe extern "system" fn on_key(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && lparam != 0 {
            // SAFETY: for WH_KEYBOARD_LL with HC_ACTION, `lparam` points to
            // a KBDLLHOOKSTRUCT valid for this call.
            let event = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
            let down = event.flags & LLKHF_UP == 0;
            let extended = if event.flags & LLKHF_EXTENDED != 0 {
                0xE000
            } else {
                0
            };
            if let Some(name) = super::key_name(event.scanCode | extended) {
                WATCH.with(|watch| {
                    if let Some(w) = watch.borrow_mut().as_mut() {
                        if down && super::modifier(&name).is_none() {
                            w.keys.set_held(modifiers_held());
                        }
                        if let Some(on) = w.keys.key(&name, down) {
                            w.shared.report(Source::Hotkey, on);
                        }
                    }
                });
            }
        }
        // SAFETY: passes the event on unchanged, as a hook must.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }
}

#[cfg(target_os = "linux")]
pub(crate) use x11::Listener;

#[cfg(target_os = "linux")]
mod x11 {
    use std::sync::Arc;
    use std::thread::JoinHandle;

    use x11rb::connection::Connection as _;
    use x11rb::protocol::Event;
    use x11rb::protocol::xinput::{self, ConnectionExt as _};
    use x11rb::protocol::xproto::{self, ConnectionExt as _};
    use x11rb::rust_connection::RustConnection;

    use super::KeyWatch;
    use crate::binding::KeyBinding;
    use crate::{PttEvent, Shared, Source};

    /// The X11 connection and the thread reading its raw key events.
    /// Dropping it wakes the thread with a message to `window` and joins it.
    pub(crate) struct Listener {
        conn: Arc<RustConnection>,
        window: xproto::Window,
        thread: Option<JoinHandle<()>>,
    }

    impl Listener {
        /// Listens to every keyboard's raw key events and reports `Ready`;
        /// `Err` with the reason when that cannot be (nothing reported).
        pub(crate) fn start(key: KeyBinding, shared: Arc<Shared>) -> Result<Self, String> {
            let (conn, window) = listen()?;
            let conn = Arc::new(conn);
            let reader = conn.clone();
            let events = shared.clone();
            let thread = std::thread::Builder::new()
                .name("ptt-keys".into())
                .spawn(move || run(&reader, window, key, &events))
                .map_err(|e| e.to_string())?;
            shared.notify(PttEvent::Ready {
                source: Source::Hotkey,
                trigger: key.to_string(),
            });
            Ok(Self {
                conn,
                window,
                thread: Some(thread),
            })
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let wake = xproto::ClientMessageEvent::new(
                32,
                self.window,
                xproto::AtomEnum::NONE,
                [0_u32; 5],
            );
            // An error means the connection is gone, and the thread with it.
            let _ = self
                .conn
                .send_event(false, self.window, xproto::EventMask::NO_EVENT, wake);
            let _ = self.conn.flush();
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    /// Connects, checks for XInput 2.1 (raw events reach every client only
    /// from 2.1), makes the window `Drop` wakes the thread through, and asks
    /// for the master keyboards' raw key events on the root window.
    fn listen() -> Result<(RustConnection, xproto::Window), String> {
        let text = |e: &dyn std::fmt::Display| e.to_string();
        let (conn, screen) = x11rb::connect(None).map_err(|e| text(&e))?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .ok_or("the display has no such screen")?
            .root;
        let version = conn
            .xinput_xi_query_version(2, 2)
            .map_err(|e| text(&e))?
            .reply()
            .map_err(|e| text(&e))?;
        if (version.major_version, version.minor_version) < (2, 1) {
            return Err(format!(
                "the X server has XInput {}.{}; 2.1 is needed",
                version.major_version, version.minor_version
            ));
        }
        let window = conn.generate_id().map_err(|e| text(&e))?;
        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            xproto::WindowClass::INPUT_ONLY,
            x11rb::COPY_FROM_PARENT,
            &xproto::CreateWindowAux::new(),
        )
        .map_err(|e| text(&e))?
        .check()
        .map_err(|e| text(&e))?;
        conn.xinput_xi_select_events(
            root,
            &[xinput::EventMask {
                deviceid: xinput::Device::ALL_MASTER.into(),
                mask: vec![
                    xinput::XIEventMask::RAW_KEY_PRESS | xinput::XIEventMask::RAW_KEY_RELEASE,
                ],
            }],
        )
        .map_err(|e| text(&e))?
        .check()
        .map_err(|e| text(&e))?;
        Ok((conn, window))
    }

    fn run(conn: &RustConnection, window: xproto::Window, key: KeyBinding, shared: &Shared) {
        let mut keys = KeyWatch::new(key);
        loop {
            let (keycode, down) = match conn.wait_for_event() {
                Ok(Event::XinputRawKeyPress(e)) => (e.detail, true),
                Ok(Event::XinputRawKeyRelease(e)) => (e.detail, false),
                Ok(Event::ClientMessage(e)) if e.window == window => break,
                Ok(_) => continue,
                Err(e) => {
                    shared.notify(PttEvent::Unavailable {
                        source: Source::Hotkey,
                        reason: e.to_string(),
                    });
                    break;
                }
            };
            // X11 keycodes are the kernel's (evdev) codes plus 8.
            if let Some(name) = keycode.checked_sub(8).and_then(super::key_name)
                && let Some(on) = keys.key(&name, down)
            {
                shared.report(Source::Hotkey, on);
            }
        }
        if keys.pressed() {
            shared.report(Source::Hotkey, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch(binding: &str) -> KeyWatch {
        KeyWatch::new(binding.parse().unwrap())
    }

    #[test]
    fn a_plain_key_presses_and_lets_go() {
        let mut w = watch("KeyQ");
        assert_eq!(w.key("KeyQ", true), Some(true));
        assert_eq!(w.key("KeyQ", true), None, "a repeat changes nothing");
        assert_eq!(w.key("KeyW", true), None, "another key");
        assert_eq!(w.key("KeyQ", false), Some(false));
        assert_eq!(w.key("KeyQ", false), None);
    }

    #[test]
    fn a_chord_needs_exactly_its_modifiers_and_lets_go_on_the_key() {
        let mut w = watch("ctrl+shift+KeyT");
        assert_eq!(w.key("KeyT", true), None, "no modifiers");
        assert_eq!(w.key("KeyT", false), None);
        w.key("ControlLeft", true);
        assert_eq!(w.key("KeyT", true), None, "shift missing");
        w.key("ShiftRight", true);
        assert_eq!(
            w.key("KeyT", true),
            Some(true),
            "left and right count alike"
        );
        // Letting go of a modifier first still ends it on the key's release.
        w.key("ControlLeft", false);
        assert_eq!(w.key("KeyT", false), Some(false));
        w.key("ControlLeft", true);
        w.key("AltLeft", true);
        assert_eq!(w.key("KeyT", true), None, "and no others");
    }

    #[test]
    fn a_plain_key_ignores_presses_with_modifiers() {
        // Ctrl+Q (quit, copy...) must not talk when Q alone is bound.
        let mut w = watch("KeyQ");
        w.key("ControlRight", true);
        assert_eq!(w.key("KeyQ", true), None);
        w.key("ControlRight", false);
        assert_eq!(w.key("KeyQ", true), Some(true));
    }

    #[test]
    fn the_system_state_replaces_modifiers_missed_meanwhile() {
        // Win+L: the logo key went down, its release went to the lock screen.
        let mut w = watch("KeyV");
        w.key("SuperLeft", true);
        assert_eq!(w.key("KeyV", true), None);
        w.key("KeyV", false);
        w.set_held([[false; 2]; 4]);
        assert_eq!(w.key("KeyV", true), Some(true));
        assert_eq!(w.key("KeyV", false), Some(false));
        // And one held meanwhile counts.
        w.set_held([[false, true], [false; 2], [false; 2], [false; 2]]);
        assert_eq!(w.key("KeyV", true), None, "right Ctrl is held");
    }

    #[test]
    fn the_logo_key_is_super_or_meta() {
        let mut w = watch("super+Space");
        w.key("SuperLeft", true);
        assert_eq!(w.key("Space", true), Some(true));
        assert_eq!(w.key("Space", false), Some(false));
        w.key("SuperLeft", false);
        w.key("MetaRight", true);
        assert_eq!(w.key("Space", true), Some(true));
    }

    /// Installs the real keyboard hook and stops its thread (no input is
    /// sent). X11 needs a display, so its listener is checked by hand.
    #[cfg(target_os = "windows")]
    #[test]
    fn the_windows_hook_starts_and_stops() {
        use std::sync::{Arc, Mutex};

        use crate::merge::Merger;
        use crate::{PttEvent, Shared, Source};

        let events = Arc::new(Mutex::new(Vec::new()));
        let seen = events.clone();
        let shared = Arc::new(Shared {
            merger: Mutex::new(Merger::default()),
            sink: Arc::new(move |e| seen.lock().unwrap().push(e)),
        });
        let listener = Listener::start("F13".parse().unwrap(), shared).expect("hooked");
        match events.lock().unwrap().first() {
            Some(PttEvent::Ready { source, trigger }) => {
                assert_eq!((*source, trigger.as_str()), (Source::Hotkey, "F13"));
            }
            other => panic!("{other:?}"),
        }
        drop(listener);
        assert_eq!(events.lock().unwrap().len(), 1, "nothing was pressed");
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    #[test]
    fn scancodes_name_keys_as_the_window_does() {
        // Windows: PS/2 set 1 scancodes, 0xE0 for extended keys; Linux: evdev.
        let (q, right_ctrl, space) = if cfg!(target_os = "windows") {
            (0x10, 0xE01D, 0x39)
        } else {
            (16, 97, 57)
        };
        assert_eq!(key_name(q).as_deref(), Some("KeyQ"));
        assert_eq!(key_name(right_ctrl).as_deref(), Some("ControlRight"));
        assert_eq!(key_name(space).as_deref(), Some("Space"));
        let bound: KeyBinding = "KeyQ".parse().unwrap();
        assert!(bound.is_key(&key_name(q).unwrap()));
    }
}
