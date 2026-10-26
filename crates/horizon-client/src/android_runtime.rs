//! Explicit Android renderer policy. Both modes render through Vulkan on the GPU.
use std::sync::atomic::{AtomicBool, Ordering};
static GPU_BATCHING: AtomicBool = AtomicBool::new(false);
static KEYPAD_ENABLED: AtomicBool = AtomicBool::new(false);
static KEYPAD: std::sync::Mutex<crate::keypad::Keypad> =
    std::sync::Mutex::new(crate::keypad::Keypad::empty());
pub fn configure_keypad(enabled: bool) {
    KEYPAD_ENABLED.store(enabled, Ordering::Relaxed);
    clear_keypad();
}
pub fn keypad_enabled() -> bool {
    KEYPAD_ENABLED.load(Ordering::Relaxed)
}
pub fn keypad_key(key: winit::keyboard::KeyCode, down: bool) -> bool {
    keypad_enabled() && KEYPAD.lock().unwrap().key(key, down)
}
pub fn clear_keypad() {
    KEYPAD.lock().unwrap().clear();
}
pub fn keypad_state() -> crate::gamepad::PadState {
    KEYPAD.lock().unwrap().poll()
}
static OPEN_GL: AtomicBool = AtomicBool::new(cfg!(target_pointer_width = "32"));
pub fn configure_graphics(open_gl: bool) {
    OPEN_GL.store(open_gl, Ordering::Relaxed);
}
pub fn graphics_api() -> crate::graphics::GraphicsApi {
    if OPEN_GL.load(Ordering::Relaxed) {
        crate::graphics::GraphicsApi::OpenGl
    } else {
        crate::graphics::GraphicsApi::Vulkan
    }
}
pub fn configure_gpu(batching: bool) {
    GPU_BATCHING.store(batching, Ordering::Relaxed);
}
pub fn compatibility() -> bool {
    cfg!(target_os = "android") && !GPU_BATCHING.load(Ordering::Relaxed)
}

/// Android's native entry point installs these without making the shared lib
/// depend on an Activity or on JNI. Clipboard access is only on explicit edits.
pub struct ClipboardHooks {
    pub get: fn() -> Option<String>,
    pub set: fn(&str),
}
static CLIPBOARD: std::sync::OnceLock<ClipboardHooks> = std::sync::OnceLock::new();
pub fn install_clipboard(hooks: ClipboardHooks) {
    let _ = CLIPBOARD.set(hooks);
}
pub fn clipboard_get() -> Option<String> {
    CLIPBOARD.get().and_then(|h| (h.get)())
}
pub fn clipboard_set(text: &str) {
    if let Some(h) = CLIPBOARD.get() {
        (h.set)(text);
    }
}

static RETURN_TO_LAUNCHER: AtomicBool = AtomicBool::new(false);
pub fn return_to_launcher() {
    RETURN_TO_LAUNCHER.store(true, Ordering::Release);
}
pub fn take_launcher_return() -> bool {
    RETURN_TO_LAUNCHER.swap(false, Ordering::AcqRel)
}
