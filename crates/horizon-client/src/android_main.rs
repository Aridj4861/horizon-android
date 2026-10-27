//! Android's native entry point. The simulation and renderer are shared with desktop.
#![cfg(target_os = "android")]

#[allow(dead_code, clippy::collapsible_match)]
// Shared vanilla match structure and desktop harnesses.
mod game;
#[path = "android/launcher.rs"]
mod launcher;
// Share the desktop launcher's actual search, compatibility checks, downloads,
// dependency resolution and Java content porting; GPUI itself is desktop-only.
#[allow(dead_code)]
#[path = "../../../launcher/src/account.rs"]
mod account;
#[allow(dead_code)]
#[path = "../../../launcher/src/browser.rs"]
mod browser;
#[path = "android/catalog.rs"]
mod catalog;
#[path = "android/gallery.rs"]
mod gallery;
#[path = "android/platform.rs"]
mod platform;
#[allow(dead_code)]
#[path = "../../../launcher/src/proxy.rs"]
mod proxy;
#[path = "android/secure_store.rs"]
mod secure_store;
// The shared account protocol needs only the launcher's version and data path.
mod backend {
    pub const VERSION: &str = if crate::PUBLIC_BUILD {
        "0.7.0"
    } else {
        "0.7.0-dev"
    };
    static HOME: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    pub fn initialize(home: std::path::PathBuf) {
        let _ = HOME.set(home);
    }
    pub fn game_home() -> std::path::PathBuf {
        data_dir()
    }
    pub fn game_mod_api_level() -> Option<u32> {
        Some(horizon_mods::api_level::LEVEL)
    }
    pub fn data_dir() -> std::path::PathBuf {
        HOME.get().expect("android_main sets account home").clone()
    }
}

use std::cell::RefCell;
pub(crate) const NATIVE_HOST: bool = cfg!(feature = "native-host");
static TOUCH_VISIBLE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(!NATIVE_HOST);
pub(crate) fn touch_visible() -> bool {
    TOUCH_VISIBLE.load(std::sync::atomic::Ordering::Relaxed)
}
pub(crate) fn touch_input(touch: bool) {
    if NATIVE_HOST {
        TOUCH_VISIBLE.store(touch, std::sync::atomic::Ordering::Relaxed);
    }
}
pub(crate) const PUBLIC_BUILD: bool = match option_env!("HORIZON_ANDROID_PUBLIC") {
    Some(value) => value.len() == 1 && value.as_bytes()[0] == b'1',
    None => false,
};
use winit::{
    event_loop::EventLoop,
    platform::android::{EventLoopBuilderExtAndroid, activity::AndroidApp},
};

thread_local! {
    static APP: RefCell<Option<AndroidApp>> = const { RefCell::new(None) };
}

pub(crate) fn android_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    APP.with(|app| {
        EventLoop::builder()
            .with_android_app(
                app.borrow()
                    .as_ref()
                    .expect("android_main sets APP")
                    .clone(),
            )
            .build()
    })
}

/// Development overrides for testing both of Horizon's single-player paths.
pub(crate) fn android_arguments() -> horizon_client::Result<Vec<String>> {
    let home = APP.with(|app| app.borrow().as_ref().unwrap().internal_data_path().unwrap());
    let config = catalog::Config::load(&home);
    let mut args = vec![
        "--resources".into(),
        home.join(".local/horizon-client/26.3")
            .to_string_lossy()
            .into_owned(),
    ];
    if !PUBLIC_BUILD {
        args.extend([
            "--control-socket".into(),
            home.join("control.json").to_string_lossy().into_owned(),
        ]);
    }
    if let Some(session) = crate::launcher::session_argument() {
        args.extend([
            "--session-file".into(),
            session.to_string_lossy().into_owned(),
        ]);
    }
    if !config.name.is_empty() {
        args.extend(["--name".into(), config.name.clone()]);
    }
    if !config.server.is_empty() {
        args.extend(["--server".into(), config.server.clone()]);
    }
    if !PUBLIC_BUILD {
        match std::fs::read(home.join("android-launch.json")) {
            Ok(bytes) => args.extend(serde_json::from_slice::<Vec<String>>(&bytes)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(args)
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    use winit::platform::android::activity::input::Axis;
    for axis in [
        Axis::Vscroll,
        Axis::Hscroll,
        Axis::RelativeX,
        Axis::RelativeY,
    ] {
        app.enable_motion_axis(axis);
    }
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("Horizon"),
    );
    let Some(home) = app.internal_data_path() else {
        log::error!("Android supplied no application data directory");
        return;
    };
    backend::initialize(home.clone());
    if let Ok(vm) = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) } {
        secure_store::initialize(vm);
    }
    let engine_data = app.asset_manager().open(c"engine-data.zip");
    APP.with(|slot| *slot.borrow_mut() = Some(app));
    horizon_client::android_runtime::install_clipboard(
        horizon_client::android_runtime::ClipboardHooks {
            get: platform::clipboard_get,
            set: platform::clipboard_set,
        },
    );
    if let Err(error) =
        std::fs::create_dir_all(&home).and_then(|()| std::env::set_current_dir(&home))
    {
        log::error!("Cannot open {}: {error}", home.display());
        return;
    }
    #[cfg(debug_assertions)]
    if secure_store::development_probe() {
        std::process::exit(0);
    }
    horizon_client::crash::install(home.clone(), "client");
    let _ = std::fs::remove_file(home.join("android-error.txt"));
    if let Some(mut data) = engine_data {
        let copied = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(home.join("engine-data.partial"))?;
            std::io::copy(&mut data, &mut file)?;
            drop(file);
            std::fs::rename(
                home.join("engine-data.partial"),
                home.join("engine-data.zip"),
            )
        })();
        if let Err(error) = copied {
            log::error!("Cannot unpack engine data: {error}");
        }
    }
    let mut launcher = launcher::Launcher::new(home.clone());
    let result = android_event_loop()
        .map_err(|e| -> Box<dyn std::error::Error> { e.into() })
        .and_then(|events| events.run_app(&mut launcher).map_err(Into::into));
    if let Err(error) = result {
        log::error!("Horizon: {error}");
        horizon_client::crash::report_error(&home, "client", &*error);
        let _ = std::fs::write(home.join("android-error.txt"), error.to_string());
    }
    // Winit permits one EventLoop per process. NativeActivity otherwise keeps
    // this process cached after Quit, making the next launch fail. run_app's
    // exiting callback has completed the world save before we get here.
    drop(launcher);
    APP.with(|slot| *slot.borrow_mut() = None);
    std::process::exit(0);
}
