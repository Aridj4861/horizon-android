//! Android renderer and platform policy.
use std::sync::atomic::{AtomicBool, Ordering};
static ATLAS_LIMIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(16384);
/// The launcher opens the adapter before the resource worker builds the game atlas.
pub fn configure_atlas_limit(limit: u32) {
    ATLAS_LIMIT.store(limit, Ordering::Relaxed);
}
pub fn atlas_limit() -> u32 {
    ATLAS_LIMIT.load(Ordering::Relaxed)
}
static GPU_BATCHING: AtomicBool = AtomicBool::new(false);
static KEYPAD_ENABLED: AtomicBool = AtomicBool::new(false);
static KEYPAD: std::sync::LazyLock<std::sync::Mutex<crate::keypad::Keypad>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(crate::keypad::Keypad::default()));
pub fn configure_keypad_settings(settings: &crate::keypad::Settings) {
    KEYPAD.lock().unwrap().configure(settings);
    KEYPAD_ENABLED.store(settings.enabled, Ordering::Relaxed);
}
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

pub use crate::clipboard::{ClipboardHooks, clipboard_get, clipboard_set, install_clipboard};

static RETURN_TO_LAUNCHER: AtomicBool = AtomicBool::new(false);
pub fn return_to_launcher() {
    RETURN_TO_LAUNCHER.store(true, Ordering::Release);
}
pub fn take_launcher_return() -> bool {
    RETURN_TO_LAUNCHER.swap(false, Ordering::AcqRel)
}
