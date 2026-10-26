//! A native Rust installer and version screen, sharing the game's Vulkan renderer.
use crate::catalog::{Catalog, Page};
use crate::game;
#[path = "login.rs"]
mod login;
thread_local! {static SESSION:std::cell::RefCell<Option<PathBuf>>=const{std::cell::RefCell::new(None)};}
pub(crate) fn session_argument() -> Option<PathBuf> {
    SESSION.with(|s| s.borrow_mut().take())
}
#[path = "chooser.rs"]
mod chooser;
#[path = "launcher_pages.rs"]
mod pages;
use horizon_client::{
    Result,
    gfx::Renderer,
    hud::{HudAtlas, HudBatch},
    ui_theme::{Face, Ui},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowId},
};

pub(super) struct Launcher {
    home: PathBuf,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui: Option<game::UiResources>,
    pub(super) audio: Option<horizon_client::audio::Audio>,
    game: Option<Box<dyn ApplicationHandler>>,
    cursor: (f64, f64),
    install: Arc<Mutex<horizon_client::official_install::InstallProgress>>,
    worker: Option<mpsc::Receiver<std::result::Result<(), String>>>,
    error: Option<String>,
    catalog: Catalog,
    targets: Vec<([f32; 4], pages::Action)>,
    drag: Option<(f64, f64, bool)>,
    chooser: Option<chooser::Chooser>,
    login: login::Login,
    session_worker: Option<mpsc::Receiver<std::result::Result<Option<PathBuf>, String>>>,
    session_ready: bool,
    focus: Option<usize>,
}

impl Launcher {
    pub(super) fn new(home: PathBuf) -> Self {
        let catalog = Catalog::new(&home);
        Self {
            home,
            window: None,
            renderer: None,
            gui: None,
            audio: None,
            game: None,
            cursor: (0.0, 0.0),
            install: Arc::default(),
            worker: None,
            error: None,
            catalog,
            targets: vec![],
            drag: None,
            chooser: None,
            login: login::Login::new(),
            session_worker: None,
            session_ready: false,
            focus: None,
        }
    }

    fn restore_launcher(&mut self, events: &ActiveEventLoop) {
        if !horizon_client::android_runtime::take_launcher_return() {
            return;
        }
        if let Some(mut game) = self.game.take() {
            game.exiting(events);
            drop(game);
        }
        crate::platform::hide();
        horizon_client::auth::clear_current();
        self.session_ready = false;
        horizon_client::discord_presence::stop();
        horizon_client::ui_theme::reset_atlas();
        self.cursor = (0.0, 0.0);
        self.drag = None;
        self.chooser = None;
        if let Err(error) = std::env::set_current_dir(&self.home) {
            self.fail(error);
            return;
        }
        self.catalog.refresh(&self.home);
        self.login.refresh_status();
        self.catalog.page = Page::Play;
        if let Err(error) = self.open(events) {
            self.fail(error);
            events.exit();
        }
    }
    fn fail(&mut self, error: impl std::fmt::Display) {
        log::error!("{error}");
        self.error = Some(error.to_string());
        let _ = std::fs::write(self.home.join("android-error.txt"), error.to_string());
    }

    fn open(&mut self, events: &ActiveEventLoop) -> Result<()> {
        crate::platform::set_orientation(self.catalog.config.orientation);
        horizon_client::android_runtime::configure_graphics(
            self.catalog.config.renderer == crate::catalog::RendererMode::OpenGl,
        );
        if let Some(renderer) = &mut self.renderer {
            renderer.recreate_surface()?;
            let size = self.window.as_ref().unwrap().inner_size();
            renderer.resize(size.width, size.height);
            return Ok(());
        }
        let window = Arc::new(
            events.create_window(Window::default_attributes().with_title("Horizon Android"))?,
        );
        let gui = game::UiResources {
            atlas: HudAtlas::build(&self.home.join("installer"))?,
            panorama: Arc::new(horizon_client::gfx::panorama::Faces {
                size: 1,
                pixels: [0u8; 4].repeat(6),
            }),
        };
        self.renderer = Some(game::loading_renderer(
            window.clone(),
            false,
            Some(&gui),
            None,
            horizon_client::graphics::GraphicsApi::Vulkan,
        )?);
        self.window = Some(window);
        self.gui = Some(gui);
        Ok(())
    }

    fn action(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.worker.is_some() || self.session_worker.is_some() {
            return Ok(());
        }
        if horizon_client::official_install::ready(&self.home) {
            crate::platform::install_bedrock_controls(&self.home)?;
            crate::platform::hide();
            self.catalog.config.save(&self.home)?;
            horizon_client::android_runtime::configure_graphics(
                self.catalog.config.renderer == crate::catalog::RendererMode::OpenGl,
            );
            horizon_client::android_runtime::configure_gpu(
                self.catalog.config.renderer == crate::catalog::RendererMode::Vulkan,
            );
            horizon_client::android_runtime::configure_keypad(self.catalog.config.keypad_controls);
            let game_home = self.catalog.config.game_home(&self.home);
            if !self.session_ready && crate::account::Accounts::load().selected().is_some() {
                let id = self.catalog.config.microsoft_client_id.clone();
                let (sender, receiver) = mpsc::channel();
                self.session_worker = Some(receiver);
                self.catalog.status = "Preparing Microsoft session...".into();
                std::thread::spawn(move || {
                    let result = (|| {
                        login::Login::session(&id)?
                            .map(|(a, s)| {
                                crate::account::write_session_file(
                                    &game_home.join("sessions"),
                                    &a,
                                    &s,
                                )
                            })
                            .transpose()
                    })();
                    let _ = sender.send(result);
                });
                return Ok(());
            }
            self.session_ready = false;
            std::fs::create_dir_all(&game_home)?;
            std::env::set_current_dir(game_home)?;
            self.audio = None;
            let game = game::android_game()?;
            // Android has one native window; release its surface before handing it over.
            self.renderer = None;
            self.window = None;
            self.gui = None;
            horizon_client::ui_theme::reset_atlas();
            self.game = Some(game);
            self.game.as_mut().unwrap().resumed(events);
        } else {
            if !self.home.join("engine-data.zip").is_file() {
                return Err(
                    "This APK is missing its engine data. Install a complete build.".into(),
                );
            }
            self.error = None;
            *self.install.lock().unwrap() = Default::default();
            let home = self.home.clone();
            let install = self.install.clone();
            let (sender, receiver) = mpsc::channel();
            self.worker = Some(receiver);
            std::thread::spawn(move || {
                let result =
                    horizon_client::official_install::install_with_progress(&home, |update| {
                        let mut current = install.lock().unwrap();
                        let done = if current.stage == update.stage {
                            current.done.max(update.done)
                        } else {
                            update.done
                        };
                        *current =
                            horizon_client::official_install::InstallProgress { done, ..update };
                    });
                let _ = sender.send(result.map_err(|e| e.to_string()));
            });
        }
        Ok(())
    }
}

impl ApplicationHandler for Launcher {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if let Some(game) = &mut self.game {
            return game.resumed(events);
        }
        if let Err(error) = self.open(events) {
            self.fail(error);
            events.exit();
        }
    }
    fn suspended(&mut self, events: &ActiveEventLoop) {
        if let Some(game) = &mut self.game {
            return game.suspended(events);
        }
        self.drag = None;
        if let Some(renderer) = &mut self.renderer {
            renderer.release_surface();
            renderer.suspended = true;
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(game) = &mut self.game {
            game.window_event(events, id, event);
            self.restore_launcher(events);
            return;
        }
        let result = match event {
            WindowEvent::CloseRequested => {
                if self.login.visible || self.login.pending() {
                    self.login.hide();
                    return;
                }
                events.exit();
                Ok(())
            }
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
                Ok(())
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.focus = None;
                self.cursor = (position.x, position.y);
                Ok(())
            }
            WindowEvent::Touch(touch) => self.touch_event(events, touch),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.activate(events),
            WindowEvent::RedrawRequested => self.draw_pages(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if self.navigate(&event) {
                    Ok(())
                } else if self.catalog.editing.is_none()
                    && !event.repeat
                    && matches!(
                        event.logical_key,
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter)
                    )
                {
                    self.activate(events)
                } else {
                    self.type_key(event);
                    Ok(())
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.catalog
                    .edit(horizon_client::osk::Output::Text(text), &self.home);
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.fail(error);
        }
    }
    fn device_event(&mut self, events: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        if let Some(game) = &mut self.game {
            game.device_event(events, id, event);
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        if let Some(game) = &mut self.game {
            game.about_to_wait(events);
            self.restore_launcher(events);
            return;
        }
        if let Some(result) = self.worker.as_ref().and_then(|w| w.try_recv().ok()) {
            self.worker = None;
            if let Err(error) = result {
                self.fail(error);
            }
        }
        for out in crate::platform::poll() {
            if matches!(out, horizon_client::osk::Output::Enter) {
                crate::platform::hide();
            }
            self.catalog.edit(out, &self.home);
        }
        if let Some(audio) = &mut self.audio {
            audio.tick(false, &Default::default());
        }
        self.catalog.poll(&self.home);
        self.login.poll();
        if let Some(result) = self.session_worker.as_ref().and_then(|w| w.try_recv().ok()) {
            self.session_worker = None;
            match result {
                Ok(file) => {
                    SESSION.with(|s| *s.borrow_mut() = file);
                    self.session_ready = true;
                    if let Err(e) = self.action(events) {
                        self.fail(e);
                    }
                }
                Err(e) => self.fail(e),
            }
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
        events.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(100),
        ));
    }
    fn exiting(&mut self, events: &ActiveEventLoop) {
        if let Some(game) = &mut self.game {
            game.exiting(events);
        }
    }
}
