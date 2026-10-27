//! Mobile launcher state. Browser operations reuse the desktop backend unchanged.
use crate::browser::{self, Kind, Project, Query, Source, installed::Installed};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

/// Temporary dev registration published as Minecraft Nintendo Switch by PrismarineJS.
/// Account tokens and client secrets are never bundled.
pub const MICROSOFT_CLIENT_ID: &str = "00000000441cc96b";

/// Real Vulkan rendering paths; these do not select a different driver or API.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RendererMode {
    Vulkan,
    OpenGl,
    #[default]
    VulkanCompatibility,
}
impl RendererMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vulkan => "Vulkan / GPU batching",
            Self::OpenGl => "OpenGL ES / experimental",
            Self::VulkanCompatibility => "Vulkan / compatibility",
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub profile: u32,
    pub proxy: Option<String>,
    pub name: String,
    pub server: String,
    #[serde(skip)]
    pub microsoft_client_id: String,
    pub renderer: RendererMode,
    pub orientation: Orientation,
    pub input_host: InputHost,
    pub keypad_controls: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InputHost {
    #[default]
    Auto,
    Touch,
    KeyboardMouse,
}
impl InputHost {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Touch, Self::KeyboardMouse];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Touch => "Touch & Android keyboard",
            Self::KeyboardMouse => "Keyboard & mouse",
        }
    }
    pub fn native(self) -> bool {
        self == Self::KeyboardMouse
            || (self == Self::Auto && crate::platform::keyboard_mouse_connected())
    }
}

/// Android ActivityInfo orientations, independent of the device's natural axis.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation {
    #[default]
    Landscape,
    Portrait,
    ReverseLandscape,
    ReversePortrait,
    Auto,
}
impl Orientation {
    pub const ALL: [Self; 5] = [
        Self::Landscape,
        Self::Portrait,
        Self::ReverseLandscape,
        Self::ReversePortrait,
        Self::Auto,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Landscape => "Landscape (sensor)",
            Self::Portrait => "Portrait (sensor)",
            Self::ReverseLandscape => "Landscape (reversed)",
            Self::ReversePortrait => "Portrait (reversed)",
            Self::Auto => "Automatic rotation",
        }
    }
    pub fn android_value(self) -> i32 {
        match self {
            Self::Landscape => 6,
            Self::Portrait => 7,
            Self::ReverseLandscape => 8,
            Self::ReversePortrait => 9,
            Self::Auto => 10,
        }
    }
}
impl Config {
    pub fn load(home: &Path) -> Self {
        let value: serde_json::Value = std::fs::read(home.join("android-launcher.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let mut config: Self = serde_json::from_value(value.clone()).unwrap_or_default();
        if cfg!(target_pointer_width = "32") && value.get("renderer").is_none() {
            config.renderer = RendererMode::OpenGl;
        }
        // Ignore saved app registrations while this dev build uses the fixed ID.
        config.microsoft_client_id = MICROSOFT_CLIENT_ID.into();
        if value.get("renderer").is_none() && value["gpu_batching"].as_bool() == Some(true) {
            config.renderer = RendererMode::Vulkan;
        }
        config
    }
    pub fn save(&self, home: &Path) -> Result<(), String> {
        let part = home.join("android-launcher.partial");
        std::fs::write(
            &part,
            serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(part, home.join("android-launcher.json")).map_err(|e| e.to_string())
    }
    pub fn profile_home(&self, home: &Path) -> PathBuf {
        if self.profile == 0 {
            home.into()
        } else {
            home.join("profiles")
                .join(format!("profile-{}", self.profile))
        }
    }
    pub fn game_home(&self, home: &Path) -> PathBuf {
        // Legacy modpack selections are ignored; their files remain untouched.
        self.profile_home(home)
    }
}
#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Play,
    Mods,
    Packs,
    Installed,
    Settings,
}
impl Page {
    pub const ALL: [Self; 5] = [
        Self::Play,
        Self::Mods,
        Self::Packs,
        Self::Installed,
        Self::Settings,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::Mods => ".hmod",
            Self::Packs => "Packs",
            Self::Installed => "Installed",
            Self::Settings => "Settings",
        }
    }
    pub fn is_catalog(self) -> bool {
        self == Self::Packs
    }
}
enum Reply {
    Search(Vec<Project>),
    Done(String),
}
pub struct Catalog {
    pub native: Vec<(String, String, bool)>,
    pub page: Page,
    pub query: String,
    pub results: Vec<Project>,
    pub status: String,
    pub offset: usize,
    pub scroll: f32,
    pub installed: Installed,
    pub config: Config,
    pub enabled: Vec<bool>,
    pub editing: Option<&'static str>,
    worker: Option<mpsc::Receiver<Result<Reply, String>>>,
}
impl Catalog {
    pub fn new(home: &Path) -> Self {
        let config = Config::load(home);
        let installed = Installed::load(&config.game_home(home));
        let mut result = Self {
            native: vec![],
            enabled: vec![],
            page: Page::Play,
            query: String::new(),
            results: vec![],
            status: String::new(),
            offset: 0,
            scroll: 0.0,
            installed,
            config,
            editing: None,
            worker: None,
        };
        result.refresh(home);
        result
    }
    pub fn refresh(&mut self, home: &Path) {
        let game = self.config.game_home(home);
        self.installed = Installed::load(&game);
        self.installed
            .entries
            .retain(|entry| entry.kind == Kind::ResourcePack);
        let on = horizon_client::mod_list::ModList::load(&game);
        let settings = horizon_client::settings::Settings::load(&game.join("options.txt"));
        self.enabled = self
            .installed
            .entries
            .iter()
            .map(|e| match e.kind {
                Kind::Mod => e
                    .file
                    .strip_prefix("horizon-mods/")
                    .is_some_and(|id| on.enabled.contains(id)),
                Kind::ResourcePack => settings.resource_packs.contains(&format!(
                    "file/{}",
                    Path::new(&e.file)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )),
                _ => false,
            })
            .collect();
        self.native = horizon_mods::find(&game.join("horizon-mods"))
            .into_iter()
            .filter(|p| p.is_file())
            .filter_map(|p| horizon_mods::Package::read(&p).ok())
            .map(|p| {
                let m = p.manifest;
                let enabled = on.enabled.contains(&m.id);
                (m.id, m.name, enabled)
            })
            .collect();
        if !cfg!(feature = "mods") {
            self.native.clear();
        }
    }
    pub fn busy(&self) -> bool {
        self.worker.is_some()
    }
    fn job(&mut self, status: &str, job: impl FnOnce() -> Result<Reply, String> + Send + 'static) {
        if self.busy() {
            return;
        }
        self.status = status.into();
        let (tx, rx) = mpsc::channel();
        self.worker = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send(job());
        });
    }
    pub fn poll(&mut self, home: &Path) {
        let reply = self.worker.as_ref().and_then(|rx| match rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("Launcher worker stopped".into())),
            Err(_) => None,
        });
        if let Some(reply) = reply {
            self.worker = None;
            match reply {
                Ok(Reply::Search(items)) => {
                    self.status = format!(
                        "{} results - select Install to check compatibility",
                        items.len()
                    );
                    self.results = items;
                }
                Ok(Reply::Done(s)) => self.status = s,
                Err(e) => self.status = e,
            }
            self.refresh(home);
        }
    }
    pub fn search(&mut self, home: &Path) {
        if self.page == Page::Mods {
            self.results.clear();
            if self.query.starts_with("https://") {
                self.import_native(home);
            } else {
                self.status = "Enter a direct HTTPS URL to a native .hmod package.".into();
            }
            return;
        }
        if !self.page.is_catalog() {
            return;
        }
        let mut query = Query::new(&self.query, Kind::ResourcePack);
        query.everything = true;
        query.offset = self.offset;
        query.limit = 12;
        self.scroll = 0.0;
        self.job("Searching Modrinth...", move || {
            browser::modrinth::Modrinth
                .search(&query)
                .map(Reply::Search)
        });
    }
    pub fn import_native(&mut self, home: &Path) {
        let url = self.query.clone();
        let game = self.config.game_home(home);
        self.job("Downloading native .hmod over HTTPS...",move||{
            use std::io::Read;
            let mut bytes=Vec::new();
            ureq::AgentBuilder::new().https_only(true).timeout_connect(std::time::Duration::from_secs(20)).timeout_read(std::time::Duration::from_secs(90)).build().get(&url).call().map_err(|e|e.to_string())?.into_reader().take(32*1024*1024+1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
            if bytes.len()>32*1024*1024 {return Err("Native mod exceeds 32 MiB".into());}
            let dir=game.join("horizon-mods");std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
            let stage=dir.join("import.partial");std::fs::write(&stage,bytes).map_err(|e|e.to_string())?;
            let result=(||{
                let package=horizon_mods::Package::read(&stage)?;
                let destination=dir.join(format!("{}.hmod",package.manifest.id));
                if destination.exists(){std::fs::copy(&destination,destination.with_extension("hmod.previous")).map_err(|e|e.to_string())?;}
                std::fs::rename(&stage,destination).map_err(|e|e.to_string())?;
                Ok(Reply::Done(format!("Installed {}. Enable it in Installed after reviewing its capabilities in game Mods.",package.manifest.name)))
            })();
            let _=std::fs::remove_file(stage);result
        });
    }
    pub fn toggle_native(&mut self, home: &Path, index: usize) {
        let Some((id, _, enabled)) = self.native.get(index) else {
            return;
        };
        let game = self.config.game_home(home);
        let mut list = horizon_client::mod_list::ModList::load(&game);
        if *enabled {
            list.enabled.remove(id);
        } else {
            list.enabled.insert(id.clone());
        }
        list.save(&game);
        self.refresh(home);
    }
    pub fn install(&mut self, home: &Path, index: usize) {
        let Some(project) = self.results.get(index).cloned() else {
            return;
        };
        let game = self.config.game_home(home);
        if project.kind != Kind::ResourcePack {
            return;
        }
        self.job("Checking compatibility and downloading...", move || {
            std::fs::create_dir_all(&game).map_err(|e| e.to_string())?;
            let source = browser::modrinth::Modrinth;
            let entries =
                browser::installed::install(&source, &project, true, &game, &|name, p| {
                    log::info!("{name}: {:.0}%", p * 100.0)
                })?;
            Ok(Reply::Done(format!(
                "Installed {} item(s). Open Play to load this profile.",
                entries.len()
            )))
        });
    }
    pub fn remove(&mut self, home: &Path, index: usize) {
        if self.busy() {
            return;
        }
        let Some(entry) = self.installed.entries.get(index) else {
            return;
        };
        let game = self.config.game_home(home);
        self.status = match browser::installed::remove(&game, &entry.project) {
            Ok(e) => {
                if e.kind == Kind::ResourcePack {
                    let path = game.join("options.txt");
                    let mut s = horizon_client::settings::Settings::load(&path);
                    let name = format!(
                        "file/{}",
                        Path::new(&e.file)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    );
                    s.resource_packs.retain(|p| p != &name);
                    s.save(&path);
                }
                format!("Removed {}", e.title)
            }
            Err(e) => e,
        };
        self.refresh(home);
    }
    pub fn toggle(&mut self, home: &Path, index: usize) {
        let Some(entry) = self.installed.entries.get(index) else {
            return;
        };
        let game = self.config.game_home(home);
        if entry.kind == Kind::ResourcePack {
            let path = game.join("options.txt");
            let mut settings = horizon_client::settings::Settings::load(&path);
            let name = format!(
                "file/{}",
                Path::new(&entry.file)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
            if settings.resource_packs.contains(&name) {
                settings.resource_packs.retain(|p| p != &name);
            } else {
                settings.resource_packs.push(name);
            }
            settings.save(&path);
        }
        self.status = format!("Updated {} for next launch", entry.title);
        self.refresh(home);
    }
    pub fn edit(&mut self, out: horizon_client::osk::Output, home: &Path) {
        use horizon_client::osk::Output;
        let field = match self.editing {
            Some("name") => &mut self.config.name,
            Some("server") => &mut self.config.server,
            Some("client_id") => &mut self.config.microsoft_client_id,
            _ => &mut self.query,
        };
        match out {
            Output::Text(s) => field.push_str(&s),
            Output::Backspace => {
                field.pop();
            }
            Output::Enter | Output::Close => {
                self.editing = None;
                let _ = self.config.save(home);
                if self.page.is_catalog() {
                    self.offset = 0;
                    self.search(home);
                }
            }
        }
    }
}
