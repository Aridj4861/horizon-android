//! System IME for NativeActivity, using stock EditText on Android's UI looper.
//! No project Java classes or cross-thread View operations are required.
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JObject, JValue},
};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::net::UnixStream},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
};

#[link(name = "android")]
unsafe extern "C" {
    fn ALooper_forThread() -> *mut std::ffi::c_void;
    fn ALooper_addFd(
        looper: *mut std::ffi::c_void,
        fd: i32,
        ident: i32,
        events: i32,
        callback: Option<unsafe extern "C" fn(i32, i32, *mut std::ffi::c_void) -> i32>,
        data: *mut std::ffi::c_void,
    ) -> i32;
}
enum Command {
    Show(String),
    Hide,
    Poll,
    Capture(bool),
    Bounds,
    ImportDocument,
}
struct Ui {
    vm: JavaVM,
    edit: GlobalRef,
    input: GlobalRef,
    native: GlobalRef,
    activity: GlobalRef,
    commands: Mutex<VecDeque<Command>>,
    reader: Mutex<UnixStream>,
    writer: Mutex<UnixStream>,
    text: Mutex<String>,
    polling: AtomicBool,
    active: AtomicBool,
    ready: AtomicBool,
    bounds_pending: AtomicBool,
    bottom: AtomicI32,
    failed: AtomicBool,
    keyboard_request: Mutex<Option<std::time::Instant>>,
}
static UI: OnceLock<Arc<Ui>> = OnceLock::new();

/// Must run during NativeActivity onCreate, on the Java UI thread.
///
/// # Safety
/// VM and activity pointers must be supplied by android-activity and valid for
/// the duration of this call. No local JNI reference is retained.
pub unsafe fn initialize(
    vm: *mut jni::sys::JavaVM,
    activity: jni::sys::jobject,
) -> Result<(), String> {
    if UI.get().is_some() {
        return Ok(());
    }
    let vm = unsafe { JavaVM::from_raw(vm) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let (edit, input, native, activity) = env
        .with_local_frame(32, |env| -> jni::errors::Result<_> {
            let activity = unsafe { JObject::from_raw(activity) };
            crate::document_bridge::register(env, &activity, crate::documents::document_result)?;
            let content = env
                .call_method(
                    &activity,
                    "findViewById",
                    "(I)Landroid/view/View;",
                    &[JValue::Int(0x01020002)],
                )?
                .l()?;
            // NativeActivity's first content child owns the native input queue.
            let native = env
                .call_method(
                    &content,
                    "getChildAt",
                    "(I)Landroid/view/View;",
                    &[JValue::Int(0)],
                )?
                .l()?;
            let edit = env.new_object(
                "android/widget/EditText",
                "(Landroid/content/Context;)V",
                &[JValue::Object(&activity)],
            )?;
            env.call_method(&edit, "setInputType", "(I)V", &[JValue::Int(0x20001)])?;
            env.call_method(&edit, "setImeOptions", "(I)V", &[JValue::Int(0x12000000)])?;
            env.call_method(&edit, "setAlpha", "(F)V", &[JValue::Float(0.0)])?;
            env.call_method(&edit, "setSaveEnabled", "(Z)V", &[JValue::Bool(0)])?;
            env.call_method(&edit, "setFocusableInTouchMode", "(Z)V", &[JValue::Bool(1)])?;
            env.call_method(&edit, "setFocusable", "(Z)V", &[JValue::Bool(0)])?;
            let params = env.new_object(
                "android/view/ViewGroup$LayoutParams",
                "(II)V",
                &[JValue::Int(1), JValue::Int(1)],
            )?;
            env.call_method(
                &activity,
                "addContentView",
                "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
                &[JValue::Object(&edit), JValue::Object(&params)],
            )?;
            let service = JObject::from(env.new_string("input_method")?);
            let input = env
                .call_method(
                    &activity,
                    "getSystemService",
                    "(Ljava/lang/String;)Ljava/lang/Object;",
                    &[JValue::Object(&service)],
                )?
                .l()?;
            env.call_method(
                &native,
                "setFocusableInTouchMode",
                "(Z)V",
                &[JValue::Bool(1)],
            )?;
            env.call_method(&native, "requestFocus", "()Z", &[])?;
            Ok((
                env.new_global_ref(edit)?,
                env.new_global_ref(input)?,
                env.new_global_ref(native)?,
                env.new_global_ref(activity)?,
            ))
        })
        .map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })?;
    drop(env);
    let (reader, writer) = UnixStream::pair().map_err(|e| e.to_string())?;
    reader.set_nonblocking(true).map_err(|e| e.to_string())?;
    writer.set_nonblocking(true).map_err(|e| e.to_string())?;
    let ui = Arc::new(Ui {
        vm,
        edit,
        input,
        native,
        activity,
        commands: Mutex::new(VecDeque::new()),
        reader: Mutex::new(reader),
        writer: Mutex::new(writer),
        text: Mutex::new(String::new()),
        polling: AtomicBool::new(false),
        active: AtomicBool::new(false),
        ready: AtomicBool::new(false),
        bounds_pending: AtomicBool::new(false),
        bottom: AtomicI32::new(0),
        failed: AtomicBool::new(false),
        keyboard_request: Mutex::new(None),
    });
    let looper = unsafe { ALooper_forThread() };
    if looper.is_null() {
        return Err("No Android UI looper during onCreate".into());
    }
    let fd = ui
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_raw_fd();
    let data = Arc::into_raw(ui.clone()).cast_mut().cast();
    if unsafe { ALooper_addFd(looper, fd, -1, 1, Some(callback), data) } != 1 {
        // Registration failed: the looper never owns this reference.
        drop(unsafe { Arc::from_raw(data.cast::<Ui>()) });
        return Err("Cannot register Android UI input channel".into());
    }
    // The callback reference and these global JNI refs live for the activity's
    // process lifetime, not per edit. Activity shutdown exits that process.
    UI.set(ui).ok();
    Ok(())
}
impl Ui {
    fn enqueue(&self, c: Command) {
        self.commands
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(c);
        let _ = self
            .writer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .write(&[1]);
    }
    fn try_show_keyboard(&self, env: &mut JNIEnv<'_>) -> jni::errors::Result<()> {
        let mut request = self
            .keyboard_request
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(started) = *request else {
            return Ok(());
        };
        if !self.active.load(Ordering::Acquire) {
            *request = None;
            return Ok(());
        }
        let focused = env.call_method(&self.edit, "hasFocus", "()Z", &[])?.z()?;
        let window_focused = env
            .call_method(&self.edit, "hasWindowFocus", "()Z", &[])?
            .z()?;
        let served = focused
            && window_focused
            && env
                .call_method(
                    &self.input,
                    "isActive",
                    "(Landroid/view/View;)Z",
                    &[JValue::Object(self.edit.as_obj())],
                )?
                .z()?;
        if served
            && env
                .call_method(
                    &self.input,
                    "showSoftInput",
                    "(Landroid/view/View;I)Z",
                    &[JValue::Object(self.edit.as_obj()), JValue::Int(0)],
                )?
                .z()?
        {
            *request = None;
        } else if started.elapsed() > std::time::Duration::from_secs(2) {
            log::warn!(
                "Android keyboard was not served (view focus={focused}, window focus={window_focused}, served={served})"
            );
            *request = None;
        }
        Ok(())
    }
    fn apply(&self, env: &mut JNIEnv<'_>, c: Command) -> jni::errors::Result<()> {
        match c {
            Command::ImportDocument => {
                let result = (|| -> jni::errors::Result<()> {
                    let action =
                        JObject::from(env.new_string("android.intent.action.OPEN_DOCUMENT")?);
                    let intent = env.new_object(
                        "android/content/Intent",
                        "(Ljava/lang/String;)V",
                        &[JValue::Object(&action)],
                    )?;
                    let category =
                        JObject::from(env.new_string("android.intent.category.OPENABLE")?);
                    env.call_method(
                        &intent,
                        "addCategory",
                        "(Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&category)],
                    )?;
                    let mime = JObject::from(env.new_string("*/*")?);
                    env.call_method(
                        &intent,
                        "setType",
                        "(Ljava/lang/String;)Landroid/content/Intent;",
                        &[JValue::Object(&mime)],
                    )?;
                    env.call_method(
                        &intent,
                        "addFlags",
                        "(I)Landroid/content/Intent;",
                        &[JValue::Int(1)],
                    )?;
                    env.call_method(
                        &self.activity,
                        "startActivityForResult",
                        "(Landroid/content/Intent;I)V",
                        &[JValue::Object(&intent), JValue::Int(4101)],
                    )?;
                    Ok(())
                })();
                if let Err(error) = result {
                    let _ = env.exception_clear();
                    log::warn!("Document picker: {error}");
                    crate::documents::picker_failed("No Android document picker is available");
                }
            }
            Command::Show(text) => {
                let s = JObject::from(env.new_string(&text)?);
                env.call_method(
                    &self.edit,
                    "setText",
                    "(Ljava/lang/CharSequence;)V",
                    &[JValue::Object(&s)],
                )?;
                env.call_method(
                    &self.edit,
                    "setSelection",
                    "(I)V",
                    &[JValue::Int(text.encode_utf16().count() as i32)],
                )?;
                // Re-enable touch focus on every edit. NativeActivity owns focus
                // between editors, and some Android versions do not serve a view
                // until the next UI-loop turn after requestFocus.
                env.call_method(&self.native, "clearFocus", "()V", &[])?;
                env.call_method(
                    &self.edit,
                    "setFocusableInTouchMode",
                    "(Z)V",
                    &[JValue::Bool(1)],
                )?;
                env.call_method(&self.edit, "requestFocus", "()Z", &[])?;
                env.call_method(
                    &self.input,
                    "restartInput",
                    "(Landroid/view/View;)V",
                    &[JValue::Object(self.edit.as_obj())],
                )?;
                *self
                    .keyboard_request
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some(std::time::Instant::now());
                self.try_show_keyboard(env)?;
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
                self.ready.store(true, Ordering::Release);
            }
            Command::Hide => {
                *self
                    .keyboard_request
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
                let token = env
                    .call_method(&self.edit, "getWindowToken", "()Landroid/os/IBinder;", &[])?
                    .l()?;
                env.call_method(
                    &self.input,
                    "hideSoftInputFromWindow",
                    "(Landroid/os/IBinder;I)Z",
                    &[JValue::Object(&token), JValue::Int(0)],
                )?;
                env.call_method(&self.edit, "clearFocus", "()V", &[])?;
                env.call_method(&self.edit, "setFocusable", "(Z)V", &[JValue::Bool(0)])?;
                env.call_method(&self.native, "requestFocus", "()Z", &[])?;
            }
            Command::Poll => {
                self.polling.store(false, Ordering::Release);
                self.try_show_keyboard(env)?;
                let editable = env
                    .call_method(&self.edit, "getText", "()Landroid/text/Editable;", &[])?
                    .l()?;
                let text = env
                    .call_method(editable, "toString", "()Ljava/lang/String;", &[])?
                    .l()?;
                let text: String = env.get_string(&text.into())?.into();
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
            }
            Command::Bounds => {
                self.bounds_pending.store(false, Ordering::Release);
                let rect = env.new_object("android/graphics/Rect", "()V", &[])?;
                env.call_method(
                    &self.native,
                    "getWindowVisibleDisplayFrame",
                    "(Landroid/graphics/Rect;)V",
                    &[JValue::Object(&rect)],
                )?;
                self.bottom
                    .store(env.get_field(rect, "bottom", "I")?.i()?, Ordering::Release);
            }
            Command::Capture(capture) => {
                env.call_method(
                    &self.native,
                    if capture {
                        "requestPointerCapture"
                    } else {
                        "releasePointerCapture"
                    },
                    "()V",
                    &[],
                )?;
            }
        }
        Ok(())
    }
}
unsafe extern "C" fn callback(_: i32, _: i32, data: *mut std::ffi::c_void) -> i32 {
    let ui = unsafe { &*data.cast::<Ui>() };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback_inner(ui))).unwrap_or_else(
        |_| {
            ui.failed.store(true, Ordering::Release);
            ui.active.store(false, Ordering::Release);
            winit::platform::android::set_native_text_input_active(false);
            log::error!("Android keyboard UI callback failed");
            0
        },
    )
}
fn callback_inner(ui: &Ui) -> i32 {
    let mut bytes = [0; 256];
    while ui
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .read(&mut bytes)
        .is_ok_and(|n| n > 0)
    {}
    let commands: Vec<_> = ui
        .commands
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .drain(..)
        .collect();
    match ui.vm.attach_current_thread() {
        Ok(mut env) => {
            for c in commands {
                if let Err(e) = env.with_local_frame(16, |env| ui.apply(env, c)) {
                    let _ = env.exception_clear();
                    log::error!("IME command: {e}");
                    ui.failed.store(true, Ordering::Release);
                    ui.active.store(false, Ordering::Release);
                    winit::platform::android::set_native_text_input_active(false);
                    return 0;
                }
            }
        }
        Err(e) => {
            log::error!("IME attach: {e}");
            ui.failed.store(true, Ordering::Release);
            ui.active.store(false, Ordering::Release);
            winit::platform::android::set_native_text_input_active(false);
            return 0;
        }
    }
    1
}
pub fn show(text: &str) -> bool {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        winit::platform::android::set_native_text_input_active(true);
        // Do not diff an earlier editor buffer while Show is queued on the UI thread.
        ui.ready.store(false, Ordering::Release);
        ui.active.store(true, Ordering::Release);
        ui.enqueue(Command::Show(text.into()));
        true
    } else {
        false
    }
}
pub fn hide() {
    winit::platform::android::set_native_text_input_active(false);
    if let Some(ui) = UI.get() {
        ui.active.store(false, Ordering::Release);
        ui.enqueue(Command::Hide)
    }
}
pub fn text() -> Option<String> {
    let ui = UI.get()?;
    if !ui.active.load(Ordering::Acquire) || !ui.ready.load(Ordering::Acquire) {
        return None;
    }
    if !ui.polling.swap(true, Ordering::AcqRel) {
        ui.enqueue(Command::Poll)
    }
    Some(
        ui.text
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
}

/// View operations, including pointer capture, belong to the Java UI thread.
pub fn capture_mouse(capture: bool) -> bool {
    if let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) {
        ui.enqueue(Command::Capture(capture));
        true
    } else {
        false
    }
}

/// Launcher-only safe bounds; View geometry is read on the Android UI thread.
pub fn viewport_bottom() -> Option<u32> {
    let ui = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire))?;
    if !ui.bounds_pending.swap(true, Ordering::AcqRel) {
        ui.enqueue(Command::Bounds);
    }
    let bottom = ui.bottom.load(Ordering::Acquire);
    (bottom > 0)
        .then(|| (bottom as f64 * winit::platform::android::native_input_scale()[1]).round() as u32)
}
pub fn pick_document() -> bool {
    let Some(ui) = UI.get().filter(|ui| !ui.failed.load(Ordering::Acquire)) else {
        return false;
    };
    ui.enqueue(Command::ImportDocument);
    true
}
