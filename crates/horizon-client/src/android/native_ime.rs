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
        atomic::{AtomicBool, Ordering},
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
}
struct Ui {
    vm: JavaVM,
    edit: GlobalRef,
    input: GlobalRef,
    native: GlobalRef,
    commands: Mutex<VecDeque<Command>>,
    reader: Mutex<UnixStream>,
    writer: Mutex<UnixStream>,
    text: Mutex<String>,
    polling: AtomicBool,
    active: AtomicBool,
    ready: AtomicBool,
    failed: AtomicBool,
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
    let (edit, input, native) = env
        .with_local_frame(32, |env| -> jni::errors::Result<_> {
            let activity = unsafe { JObject::from_raw(activity) };
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
            env.call_method(&native, "requestFocus", "()Z", &[])?;
            Ok((
                env.new_global_ref(edit)?,
                env.new_global_ref(input)?,
                env.new_global_ref(native)?,
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
        commands: Mutex::new(VecDeque::new()),
        reader: Mutex::new(reader),
        writer: Mutex::new(writer),
        text: Mutex::new(String::new()),
        polling: AtomicBool::new(false),
        active: AtomicBool::new(false),
        ready: AtomicBool::new(false),
        failed: AtomicBool::new(false),
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
    fn apply(&self, env: &mut JNIEnv<'_>, c: Command) -> jni::errors::Result<()> {
        match c {
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
                env.call_method(&self.edit, "setFocusable", "(Z)V", &[JValue::Bool(1)])?;
                env.call_method(&self.edit, "requestFocus", "()Z", &[])?;
                let shown = env
                    .call_method(
                        &self.input,
                        "showSoftInput",
                        "(Landroid/view/View;I)Z",
                        &[JValue::Object(self.edit.as_obj()), JValue::Int(0)],
                    )?
                    .z()?;
                *self
                    .text
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = text;
                self.ready.store(true, Ordering::Release);
                if !shown {
                    log::debug!("Android has deferred the system keyboard request");
                }
            }
            Command::Hide => {
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
