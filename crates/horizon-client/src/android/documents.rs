//! SAF selection, bounded background imports and explicit collision choices.
use horizon_client::{
    Result,
    content_files::{self, Kind},
    hud::{HudAtlas, HudBatch, rgba},
};
use jni::{
    JNIEnv,
    objects::{JObject, JString, JValue},
};
use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

struct Request {
    kind: Kind,
    home: PathBuf,
    cache: PathBuf,
}
struct Conflict {
    kind: Kind,
    staged: PathBuf,
    target: PathBuf,
}
#[derive(Clone)]
pub struct Notice {
    pub text: String,
    pub changed: bool,
}
#[derive(Default)]
struct State {
    request: Option<Request>,
    working: bool,
    conflict: Option<Conflict>,
    notice: Option<Notice>,
    hits: Vec<([f32; 4], Choice)>,
    toast: Option<(String, std::time::Instant)>,
    cursor: (f32, f32),
}
#[derive(Clone, Copy)]
pub enum Choice {
    KeepBoth,
    Replace,
    Cancel,
}
static STATE: Mutex<State> = Mutex::new(State {
    request: None,
    working: false,
    conflict: None,
    notice: None,
    hits: Vec::new(),
    toast: None,
    cursor: (0.0, 0.0),
});
static CANCEL: AtomicBool = AtomicBool::new(false);
pub fn busy() -> bool {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.request.is_some() || s.working || s.conflict.is_some()
}
pub fn conflict() -> bool {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .conflict
        .is_some()
}
fn notice(text: impl Into<String>, changed: bool) {
    let text = text.into();
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    s.toast = Some((text.clone(), std::time::Instant::now()));
    s.notice = Some(Notice { text, changed });
}
pub fn poll() -> Option<Notice> {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .notice
        .take()
}
pub fn begin(kind: Kind, home: PathBuf) -> std::result::Result<(), String> {
    if busy() {
        return Err("Finish or cancel the current import first".into());
    }
    CANCEL.store(false, Ordering::Relaxed);
    crate::platform::hide();
    let cache = crate::platform::cache_dir()
        .ok_or("Could not access the Android import cache")?
        .join("content-imports");
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = Some(Request { kind, home, cache });
    if !crate::native_ime::pick_document() {
        picker_failed("Android document picker is unavailable");
        return Err("Android document picker is unavailable".into());
    }
    notice(format!("Choose a {} file", kind.label()), false);
    Ok(())
}
pub fn picker_failed(error: &str) {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = None;
    notice(error, false);
}
/// Called by the approved Activity result bridge, never from a game tick.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_horizon_android_HorizonActivity_nativeDocumentResult(
    mut env: JNIEnv<'_>,
    activity: JObject<'_>,
    request: i32,
    result: i32,
    uri: JString<'_>,
) {
    if request != 4101 {
        return;
    }
    let pending = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .request
        .take();
    let Some(pending) = pending else {
        return;
    };
    if result != -1 || uri.is_null() {
        notice("Import cancelled", false);
        return;
    }
    let setup = (|| -> jni::errors::Result<_> {
        let text: String = env.get_string(&uri)?.into();
        let activity = env.new_global_ref(activity)?;
        Ok((env.get_java_vm()?, activity, text))
    })();
    let Ok((vm, activity, uri)) = setup else {
        let _ = env.exception_clear();
        notice("Could not read the selected document", false);
        return;
    };
    if !uri.starts_with("content://") || uri.len() > 8192 {
        notice("Unsupported document URI", false);
        return;
    }
    STATE.lock().unwrap_or_else(|e| e.into_inner()).working = true;
    let spawned=std::thread::Builder::new().name("Content import".into()).spawn(move|| {
        let result=(||->Result<()> {
            let mut env=vm.attach_current_thread()?;
            let resolver=env.call_method(activity.as_obj(),"getContentResolver","()Landroid/content/ContentResolver;",&[])?.l()?;
            let text=JObject::from(env.new_string(uri)?);
            let uri=env.call_static_method("android/net/Uri","parse","(Ljava/lang/String;)Landroid/net/Uri;",&[JValue::Object(&text)])?.l()?;
            let projection=env.new_object_array(1,"java/lang/String",JObject::null())?;
            env.set_object_array_element(&projection,0,env.new_string("_display_name")?)?;
            let cursor=env.call_method(&resolver,"query","(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",&[JValue::Object(&uri),JValue::Object(projection.as_ref()),JValue::Object(&JObject::null()),JValue::Object(&JObject::null()),JValue::Object(&JObject::null())])?.l()?;
            let name=if !cursor.is_null() {
                let name=if env.call_method(&cursor,"moveToFirst","()Z",&[])?.z()? {
                    let value=env.call_method(&cursor,"getString","(I)Ljava/lang/String;",&[JValue::Int(0)])?.l()?;
                    Some(env.get_string((&value).into())?.into())
                } else {None};
                env.call_method(&cursor,"close","()V",&[])?;
                name
            } else {None};
            let name:String=name.unwrap_or_else(||if pending.kind==Kind::Mods {"import.hmod".into()} else {"import.zip".into()});
            let mode=JObject::from(env.new_string("r")?);
            let fd=env.call_method(&resolver,"openFileDescriptor","(Landroid/net/Uri;Ljava/lang/String;)Landroid/os/ParcelFileDescriptor;",&[JValue::Object(&uri),JValue::Object(&mode)])?.l()?;
            let raw=env.call_method(&fd,"detachFd","()I",&[])?.i()?;
            if raw<0 { return Err("The document provider did not return a readable file".into()); }
            use std::os::fd::FromRawFd;
            let input=unsafe{std::fs::File::from_raw_fd(raw)};
            stage(pending,input,name)
        })();
        STATE.lock().unwrap_or_else(|e|e.into_inner()).working=false;
        if let Err(error)=result { notice(format!("Import failed: {error}"),false); }
    });
    if let Err(error) = spawned {
        STATE.lock().unwrap_or_else(|e| e.into_inner()).working = false;
        notice(format!("Import failed: {error}"), false);
    }
}
fn stage(request: Request, mut input: std::fs::File, name: String) -> Result<()> {
    if !content_files::plain_name(&name) || !request.kind.accepts(&name) {
        return Err("Selected file has the wrong extension for this folder".into());
    }
    let dir = request.home.join(request.kind.folder());
    std::fs::create_dir_all(&dir)?;
    if std::fs::symlink_metadata(&dir)?.file_type().is_symlink() {
        return Err("Refusing a redirected content directory".into());
    }
    std::fs::create_dir_all(&request.cache)?;
    if std::fs::symlink_metadata(&request.cache)?
        .file_type()
        .is_symlink()
    {
        return Err("Refusing a redirected import cache".into());
    }
    for entry in std::fs::read_dir(&request.cache)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".import-") && name.ends_with(".part") && entry.file_type()?.is_file() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let staged = request.cache.join(format!(".import-{stamp}.part"));
    let result = (|| -> Result<()> {
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        content_files::copy_bounded(&mut input, &mut output, |bytes| {
            notice(
                format!("Importing {}: {} KiB", request.kind.label(), bytes / 1024),
                false,
            );
            !CANCEL.load(Ordering::Relaxed)
        })?;
        output.sync_all()?;
        drop(output);
        notice("Checking archive...", false);
        let native_shader = content_files::validate(&staged, request.kind, &name, || {
            CANCEL.load(Ordering::Relaxed)
        })?;
        let name = if request.kind == Kind::Mods {
            format!("{}.hmod", horizon_mods::Package::read(&staged)?.manifest.id)
        } else if native_shader || name.to_ascii_lowercase().ends_with(".hshader") {
            let pack = horizon_shader_api::Pack::read(&staged)?;
            horizon_shader_api::check(&pack).map_err(|e| format!("Invalid shader pack: {e:?}"))?;
            let mut path = PathBuf::from(name);
            path.set_extension("hshader");
            path.to_string_lossy().into_owned()
        } else {
            name
        };
        if !content_files::plain_name(&name) {
            return Err("Package has an invalid ID or filename".into());
        }
        if CANCEL.load(Ordering::Relaxed) {
            return Err("Import cancelled".into());
        }
        let target = dir.join(name);
        if target.exists() {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).conflict = Some(Conflict {
                kind: request.kind,
                staged: staged.clone(),
                target,
            });
            notice(
                "File already exists: choose Keep both, Replace or Cancel",
                false,
            );
        } else {
            std::fs::rename(&staged, &target)?;
            notice(
                format!(
                    "Imported {} into {}. Enable it in game settings.",
                    target.file_name().unwrap_or_default().to_string_lossy(),
                    request.kind.label()
                ),
                true,
            );
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(staged);
    }
    result
}
pub fn resolve(choice: Choice) {
    let c = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .conflict
        .take();
    let Some(mut c) = c else {
        return;
    };
    if matches!(choice, Choice::Cancel) {
        let _ = std::fs::remove_file(c.staged);
        notice("Import cancelled", false);
        return;
    }
    let result = (|| -> Result<()> {
        if matches!(choice, Choice::Replace)
            && std::fs::symlink_metadata(&c.target)
                .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err("Cannot replace a folder or symbolic link".into());
        }
        if matches!(choice, Choice::KeepBoth) {
            if c.kind == Kind::Mods {
                return Err(
                    "A mod ID cannot be installed twice; choose Replace to update it".into(),
                );
            }
            let mut stem = c
                .target
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            while stem.len() > 150 {
                stem.pop();
            }
            let ext = c
                .target
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let dir = c
                .target
                .parent()
                .ok_or("Invalid content folder")?
                .to_owned();
            for i in 1..=10000 {
                let candidate = dir.join(format!("{stem} ({i}).{ext}"));
                if !candidate.exists() {
                    c.target = candidate;
                    break;
                }
                if i == 10000 {
                    return Err("Too many filename conflicts".into());
                }
            }
        }
        std::fs::rename(&c.staged, &c.target)?;
        Ok(())
    })();
    match result {
        Ok(()) => notice(
            "Import completed. Enable the addition in game settings.",
            true,
        ),
        Err(e) => {
            let _ = std::fs::remove_file(c.staged);
            notice(format!("Import failed: {e}"), false);
        }
    }
}
pub fn cancel() {
    CANCEL.store(true, Ordering::Relaxed);
    STATE.lock().unwrap_or_else(|e| e.into_inner()).request = None;
    if conflict() {
        resolve(Choice::Cancel);
    } else {
        notice("Import cancelled", false);
    }
}
pub fn tap(x: f32, y: f32) -> bool {
    if !busy() {
        return false;
    }
    let choice = STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .hits
        .iter()
        .find(|(r, _)| x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3])
        .map(|(_, c)| *c);
    if let Some(choice) = choice {
        if matches!(choice, Choice::Cancel) {
            cancel();
        } else {
            resolve(choice);
        }
    }
    true
}
pub fn draw(batch: &mut HudBatch, atlas: &HudAtlas) {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if s.conflict.is_none() && !s.working {
        if let Some((message, time)) = &s.toast
            && time.elapsed().as_secs() < 5
        {
            let text = fit(atlas, message, batch.width - 16.0);
            batch.fill(
                atlas,
                4.0,
                batch.height - 28.0,
                batch.width - 8.0,
                24.0,
                rgba(24, 27, 32, 235),
            );
            batch.text(
                atlas,
                &text,
                8.0,
                batch.height - 20.0,
                rgba(255, 255, 255, 255),
                false,
            );
        }
        return;
    }
    let (name, choices) = if let Some(c) = &s.conflict {
        (
            c.target
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .chars()
                .take(42)
                .collect::<String>(),
            if c.kind == Kind::Mods {
                vec![("Replace", Choice::Replace), ("Cancel", Choice::Cancel)]
            } else {
                vec![
                    ("Keep both", Choice::KeepBoth),
                    ("Replace", Choice::Replace),
                    ("Cancel", Choice::Cancel),
                ]
            },
        )
    } else {
        (
            s.toast
                .as_ref()
                .map(|v| v.0.chars().take(42).collect())
                .unwrap_or_default(),
            vec![("Cancel", Choice::Cancel)],
        )
    };
    let w = 300.0f32.min(batch.width - 8.0);
    let vertical = w < 240.0;
    let height = if vertical {
        62.0 + choices.len() as f32 * 30.0
    } else {
        100.0
    };
    let x = (batch.width - w) / 2.0;
    let y = ((batch.height - height) / 2.0).max(0.0);
    batch.fill(
        atlas,
        0.0,
        0.0,
        batch.width,
        batch.height,
        rgba(0, 0, 0, 180),
    );
    batch.fill(atlas, x, y, w, height, rgba(24, 27, 32, 255));
    batch.text(
        atlas,
        if s.conflict.is_some() {
            "File exists"
        } else {
            "Importing file"
        },
        x + 8.0,
        y + 10.0,
        rgba(255, 255, 255, 255),
        false,
    );
    batch.text(
        atlas,
        &fit(atlas, &name, w - 16.0),
        x + 8.0,
        y + 30.0,
        rgba(215, 215, 215, 255),
        false,
    );
    s.hits.clear();
    let width = (w - 16.0) / choices.len() as f32;
    for (i, (label, choice)) in choices.into_iter().enumerate() {
        let rect = if vertical {
            [x + 8.0, y + 54.0 + i as f32 * 30.0, w - 16.0, 26.0]
        } else {
            [x + 8.0 + i as f32 * width, y + 62.0, width - 4.0, 26.0]
        };
        batch.fill(
            atlas,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            rgba(65, 70, 80, 255),
        );
        batch.text(
            atlas,
            label,
            rect[0] + 4.0,
            rect[1] + 8.0,
            rgba(255, 255, 255, 255),
            false,
        );
        s.hits.push((
            [
                rect[0] * batch.scale,
                rect[1] * batch.scale,
                rect[2] * batch.scale,
                rect[3] * batch.scale,
            ],
            choice,
        ));
    }
}
fn fit(atlas: &HudAtlas, text: &str, width: f32) -> String {
    let mut text = text.chars().take(120).collect::<String>();
    while !text.is_empty() && atlas.font.width(&text) > width {
        text.pop();
    }
    text
}
pub fn handle_event(event: &winit::event::WindowEvent) -> bool {
    use winit::{
        event::{ElementState, MouseButton, TouchPhase, WindowEvent},
        keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    };
    if !busy() {
        return false;
    }
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).cursor =
                (position.x as f32, position.y as f32);
            true
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            ..
        } => {
            if *state == ElementState::Released {
                let (x, y) = STATE.lock().unwrap_or_else(|e| e.into_inner()).cursor;
                tap(x, y);
            }
            true
        }
        WindowEvent::Touch(t) => {
            if t.phase == TouchPhase::Ended {
                tap(t.location.x as f32, t.location.y as f32);
            }
            true
        }
        WindowEvent::KeyboardInput { event, .. } => {
            if event.state == ElementState::Pressed {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                    || event.logical_key == Key::Named(NamedKey::BrowserBack)
                {
                    cancel();
                } else if event.physical_key == PhysicalKey::Code(KeyCode::Enter) && conflict() {
                    let native = STATE
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .conflict
                        .as_ref()
                        .is_some_and(|c| c.kind == Kind::Mods);
                    resolve(if native {
                        Choice::Cancel
                    } else {
                        Choice::KeepBoth
                    });
                }
            }
            true
        }
        _ => false,
    }
}
