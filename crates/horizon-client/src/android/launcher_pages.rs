use super::*;
use horizon_client::{osk::Output, ui_theme::color};
use winit::{
    event::{KeyEvent, Touch},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Action {
    ToggleNative(usize),
    Page(Page),
    Play,
    Search,
    Field(&'static str),
    FinishNameEdit,
    Install(usize),
    Remove(usize),
    Toggle(usize),
    Next,
    Previous,
    Profile,
    Logs,
    Account,
    SignIn,
    DeviceSignIn,
    CopyLoginCode,
    CancelLogin,
    ReviewApp,
    Repository,
    SignOut,
    LoginLink,
    CloseLogin,
    Renderer,
    RenderScale,
    ChooseRenderScale(crate::display::RenderScale),
    Orientation,
    InputHost,
    ChooseInputHost(crate::catalog::InputHost),
    ChooseOrientation(crate::catalog::Orientation),
    ChooseRenderer(crate::catalog::RendererMode),
    ChooseProfile(u32),
    Dismiss,
}

pub(super) fn button(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    label: &str,
    action: Action,
    accent: bool,
) {
    let [x, y, w, h] = rect;
    ui.rounded(
        x,
        y,
        w,
        h,
        2.0,
        if accent { color::ACCENT } else { color::RAISED },
    );
    let padding = (w * 0.12).clamp(4.0, 12.0);
    let font = if w < 80.0 { 13.0 } else { 16.0 };
    let label = ui.fit_text(Face::Medium, font, label, w - 2.0 * padding);
    ui.text(
        Face::Medium,
        font,
        x + padding,
        y + 11.0,
        if accent {
            color::ON_ACCENT
        } else {
            color::TEXT
        },
        &label,
    );
    targets.push((rect, action));
}
fn toggle(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    rect: [f32; 4],
    on: bool,
    action: Action,
) {
    let [x, y, w, h] = rect;
    ui.text(
        Face::Regular,
        14.0,
        x,
        y + 11.0,
        color::TEXT,
        if on { "On" } else { "Off" },
    );
    ui.rounded(
        x + w - 52.0,
        y + 8.0,
        48.0,
        24.0,
        12.0,
        if on { color::TEXT } else { color::HOVER },
    );
    ui.rounded(
        x + w - if on { 26.0 } else { 50.0 },
        y + 10.0,
        20.0,
        20.0,
        10.0,
        if on { color::BG } else { color::TEXT },
    );
    targets.push(([x, y, w, h], action));
}
impl Launcher {
    /// The same spatial focus algorithm as the game's controller menus.
    pub(super) fn navigate(&mut self, key: &KeyEvent) -> bool {
        use horizon_client::gamepad::{self, Button};
        use winit::keyboard::{Key, NamedKey};
        if self.pending_focus.is_some() {
            return true;
        }
        if self.catalog.editing.is_some() || self.targets.is_empty() {
            return false;
        }
        let button = match key.logical_key {
            Key::Named(NamedKey::ArrowUp) => Button::DPadUp,
            Key::Named(NamedKey::ArrowDown) => Button::DPadDown,
            Key::Named(NamedKey::ArrowLeft) => Button::DPadLeft,
            Key::Named(NamedKey::ArrowRight) => Button::DPadRight,
            Key::Named(NamedKey::Tab) => {
                self.focus = Some(self.focus.map_or(0, |i| (i + 1) % self.targets.len()));
                return true;
            }
            _ => return false,
        };
        if matches!(button, Button::DPadUp | Button::DPadDown)
            && let Some((_, action)) = self.focus.and_then(|i| self.targets.get(i))
            && let Some(index) = self.list_targets.iter().position(|(_, a)| a == action)
        {
            let rects: Vec<_> = self.list_targets.iter().map(|(r, _)| *r).collect();
            if let Some(next) = gamepad::focus_step_strict(rects[index], button, &rects)
                && let Some(view) = self.list_view
            {
                let (rect, action) = self.list_targets[next];
                let scroll = if let Some(chooser) = &mut self.chooser {
                    &mut chooser.scroll
                } else {
                    &mut self.catalog.scroll
                };
                *scroll = super::gesture::reveal(*scroll, rect, view);
                self.pending_focus = Some(action);
                self.focus = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                return true;
            }
        }
        self.focus = Some(
            if let Some(index) = self.focus.filter(|i| *i < self.targets.len()) {
                let rects: Vec<_> = self.targets.iter().map(|(rect, _)| *rect).collect();
                gamepad::focus_step_rect(rects[index], button, &rects).unwrap_or(index)
            } else {
                0
            },
        );
        true
    }

    pub(super) fn draw_pages(&mut self) -> Result<()> {
        let (Some(r), Some(gui)) = (&mut self.renderer, &self.gui) else {
            return Ok(());
        };
        self.targets.clear();
        self.list_targets.clear();
        self.list_view = None;
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let mut ui = Ui::mobile(&gui.atlas, &mut b, (0.0, 0.0));
        // OEM navigation bars can remain visible even in fullscreen (keypad phones).
        // Inset the launcher layout only; keep surface and input coordinates intact.
        if let Some(window) = &self.window {
            use winit::platform::android::WindowExtAndroid;
            let rect = window.content_rect();
            let input = winit::platform::android::native_input_scale();
            if rect.bottom > 0 {
                ui.h = ui.h.min(rect.bottom as f32 * input[1] as f32 / ui.density);
            }
            if rect.right > 0 {
                ui.w = ui.w.min(rect.right as f32 * input[0] as f32 / ui.density);
            }
        }
        if let Some(bottom) = crate::native_ime::viewport_bottom() {
            ui.h = ui.h.min(bottom as f32 / ui.density);
        }
        let (w, h) = (ui.w, ui.h);
        let scale = ui.density / ui.batch.scale;
        ui.batch
            .fill(&gui.atlas, 0.0, 0.0, w * scale, h * scale, color::BG);
        ui.text(
            Face::Medium,
            if w < 400.0 {
                20.0
            } else if w < 700.0 {
                24.0
            } else {
                32.0
            },
            32.0,
            if w < 700.0 { 12.0 } else { 28.0 },
            color::TEXT,
            &format!("Horizon Android {}", crate::backend::VERSION),
        );
        button(
            &mut ui,
            &mut self.targets,
            [
                if w < 700.0 { 32.0 } else { w - 320.0 },
                if w < 700.0 { 47.0 } else { 24.0 },
                if w < 700.0 { w - 64.0 } else { 288.0 },
                if w < 700.0 { 34.0 } else { 46.0 },
            ],
            &self.login.status(&self.catalog.config.name),
            Action::Account,
            self.login.signed_in,
        );
        let tabs = Page::ALL.len() as f32;
        let tab_width = (w - 64.0) / tabs;
        for (i, page) in Page::ALL.iter().enumerate() {
            let x = 32.0 + i as f32 * tab_width;
            button(
                &mut ui,
                &mut self.targets,
                [x, 88.0, tab_width - 8.0, 42.0],
                if w < 700.0 {
                    match page {
                        Page::Installed => "Files",
                        Page::Settings => "Setup",
                        _ => page.name(),
                    }
                } else {
                    page.name()
                },
                Action::Page(*page),
                *page == self.catalog.page,
            );
        }
        let compact = w < 700.0;
        let page = self.catalog.page;
        if page == Page::Play {
            use horizon_client::official_install::InstallStage;
            let installed = horizon_client::official_install::ready(&self.home);
            let installing = self.worker.is_some();
            let progress = *self.install.lock().unwrap();
            let top = 160.0;
            ui.text(
                Face::Medium,
                32.0,
                32.0,
                top,
                color::TEXT,
                if installing {
                    "Getting your game ready"
                } else if installed {
                    "Ready to play"
                } else {
                    if compact {
                        "Your Java worlds"
                    } else {
                        "Your Java worlds, on Android"
                    }
                },
            );
            ui.text(
                Face::Regular,
                17.0,
                32.0,
                top + 47.0,
                color::MUTED,
                &format!("Minecraft 26.3  /  Profile {}", self.catalog.config.profile),
            );
            if installing {
                let current = InstallStage::ALL
                    .iter()
                    .position(|s| *s == progress.stage)
                    .unwrap_or(0);
                let row_height = ((h - 360.0) / 6.0).clamp(25.0, 40.0);
                if h >= 620.0 {
                    for (index, stage) in InstallStage::ALL.iter().enumerate() {
                        let y = top + 90.0 + index as f32 * row_height;
                        let label = if index < current {
                            format!("{:02}  {}  - done", index + 1, stage.label())
                        } else {
                            format!("{:02}  {}", index + 1, stage.label())
                        };
                        ui.text(
                            Face::Regular,
                            16.0,
                            32.0,
                            y,
                            if index == current {
                                color::TEXT
                            } else {
                                color::MUTED
                            },
                            &label,
                        );
                    }
                } else {
                    ui.text(
                        Face::Regular,
                        18.0,
                        32.0,
                        top + 100.0,
                        color::TEXT,
                        progress.stage.label(),
                    );
                    ui.text(
                        Face::Regular,
                        15.0,
                        32.0,
                        top + 138.0,
                        color::MUTED,
                        &format!("Step {} of 6", current + 1),
                    );
                }
                let fraction = if progress.total > 0 {
                    progress.done as f32 / progress.total as f32
                } else {
                    0.0
                };
                let overall = (current as f32 + fraction.clamp(0.0, 1.0)) / 6.0;
                let bar_y = h - 152.0;
                ui.rounded(32.0, bar_y, w - 64.0, 4.0, 2.0, color::RAISED);
                if overall > 0.0 {
                    ui.rounded(32.0, bar_y, (w - 64.0) * overall, 4.0, 2.0, color::ACCENT);
                }
                let detail = match progress.stage {
                    InstallStage::Download => format!(
                        "{:.1} / {:.1} MiB  - verified downloads",
                        progress.done as f64 / 1048576.0,
                        progress.total as f64 / 1048576.0
                    ),
                    InstallStage::Controls => format!(
                        "{} / {} control files verified",
                        progress.done, progress.total
                    ),
                    _ => "Please keep Horizon open. Downloaded files are kept for retry.".into(),
                };
                ui.text(
                    Face::Regular,
                    14.0,
                    32.0,
                    bar_y + 12.0,
                    color::MUTED,
                    &detail,
                );
            } else {
                let rows = if installed {
                    [
                        "01  Open a world or join a Java server",
                        "02  Touch setup: Options > Controls",
                        "03  Find packs and .hmod in the tabs",
                    ]
                } else {
                    [
                        "01  Download resources from Mojang",
                        "02  Install and verify on this phone",
                        "03  Play offline or sign in for servers",
                    ]
                };
                for (index, line) in rows.iter().enumerate() {
                    ui.text(
                        Face::Regular,
                        17.0,
                        32.0,
                        top + 100.0 + index as f32 * 42.0,
                        color::MUTED,
                        line,
                    );
                }
            }
            if installing {
                ui.text(
                    Face::Medium,
                    19.0,
                    32.0,
                    h - 90.0,
                    color::TEXT,
                    progress.stage.label(),
                );
            } else {
                button(
                    &mut ui,
                    &mut self.targets,
                    [32.0, h - 100.0, 320.0, 60.0],
                    if installed {
                        "Play"
                    } else if self.error.is_some() {
                        "Retry installation"
                    } else {
                        "Download Minecraft 26.3"
                    },
                    Action::Play,
                    true,
                );
            }
        } else if page.is_catalog() {
            button(
                &mut ui,
                &mut self.targets,
                [32.0, 150.0, w - 240.0, 44.0],
                &format!("Search: {}", self.catalog.query),
                Action::Field("search"),
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [w - 190.0, 150.0, 158.0, 44.0],
                "Search",
                Action::Search,
                true,
            );
            ui.text(
                Face::Regular,
                14.0,
                32.0,
                206.0,
                color::MUTED,
                if page == Page::Mods {
                    "Reviewed Horizon .hmod mods. Runtime requires a 64-bit build."
                } else {
                    "Resource packs download with checksum verification. Enable them in Installed."
                },
            );
            let top = 242.0;
            let bottom = h - 112.0;
            self.catalog.scroll = self.catalog.scroll.clamp(
                0.0,
                (self.catalog.results.len() as f32 * 78.0 - (bottom - top)).max(0.0),
            );
            self.list_view = Some([
                top,
                bottom,
                (self.catalog.results.len() as f32 * 78.0 - (bottom - top)).max(0.0),
            ]);
            for (i, p) in self.catalog.results.iter().enumerate() {
                let title = ui.fit_text(Face::Medium, 19.0, &p.title, w - 214.0);
                let summary = ui.fit_text(Face::Regular, 14.0, &p.summary, w - 214.0);
                let y = top + i as f32 * 78.0 - self.catalog.scroll;
                self.list_targets
                    .push(([w - 150.0, y, 118.0, 70.0], Action::Install(i)));
                if y < top || y + 70.0 > bottom {
                    continue;
                }
                ui.text(Face::Medium, 19.0, 32.0, y, color::TEXT, &title);
                ui.text(Face::Regular, 14.0, 32.0, y + 29.0, color::MUTED, &summary);
                if p.kind == crate::browser::Kind::Mod && !cfg!(feature = "mods") {
                    ui.text(
                        Face::Regular,
                        14.0,
                        w - 150.0,
                        y + 12.0,
                        color::MUTED,
                        "64-bit required",
                    );
                } else {
                    button(
                        &mut ui,
                        &mut self.targets,
                        [w - 150.0, y, 118.0, 42.0],
                        "Install",
                        Action::Install(i),
                        false,
                    );
                }
            }
            button(
                &mut ui,
                &mut self.targets,
                [32.0, h - 100.0, 120.0, 40.0],
                "Previous",
                Action::Previous,
                false,
            );
            button(
                &mut ui,
                &mut self.targets,
                [166.0, h - 100.0, 120.0, 40.0],
                "Next",
                Action::Next,
                false,
            );
        } else if page == Page::Installed {
            let top = 160.0;
            let bottom = h - 100.0;
            let row_height = if compact { 110.0 } else { 72.0 };
            self.catalog.scroll = self.catalog.scroll.clamp(
                0.0,
                ((self.catalog.installed.entries.len() + self.catalog.native.len()) as f32
                    * row_height
                    - (bottom - top))
                    .max(0.0),
            );
            self.list_view = Some([
                top,
                bottom,
                ((self.catalog.installed.entries.len() + self.catalog.native.len()) as f32
                    * row_height
                    - (bottom - top))
                    .max(0.0),
            ]);
            if self.catalog.installed.entries.is_empty() && self.catalog.native.is_empty() {
                ui.text(
                    Face::Regular,
                    20.0,
                    32.0,
                    180.0,
                    color::MUTED,
                    "No additions yet. Find resource packs in Packs.",
                );
            }
            for (i, e) in self.catalog.installed.entries.iter().enumerate() {
                let title = ui.fit_text(
                    Face::Medium,
                    19.0,
                    &e.title,
                    if compact { w - 64.0 } else { w - 364.0 },
                );
                let y = top + i as f32 * row_height - self.catalog.scroll;
                let row_size = if compact { 95.0 } else { 64.0 };
                self.list_targets.push((
                    [if compact { 32.0 } else { w - 300.0 }, y, 120.0, row_size],
                    Action::Toggle(i),
                ));
                self.list_targets
                    .push(([w - 165.0, y, 133.0, row_size], Action::Remove(i)));
                if y < top || y + if compact { 95.0 } else { 64.0 } > bottom {
                    continue;
                }
                ui.text(Face::Medium, 19.0, 32.0, y, color::TEXT, &title);
                ui.text(
                    Face::Regular,
                    14.0,
                    32.0,
                    y + 28.0,
                    color::MUTED,
                    &format!(
                        "{} / {}",
                        e.number,
                        e.verdict.as_deref().unwrap_or("Resource pack")
                    ),
                );
                toggle(
                    &mut ui,
                    &mut self.targets,
                    [
                        if compact { 32.0 } else { w - 300.0 },
                        y + if compact { 55.0 } else { 0.0 },
                        120.0,
                        40.0,
                    ],
                    self.catalog.enabled.get(i).copied().unwrap_or(false),
                    Action::Toggle(i),
                );
                button(
                    &mut ui,
                    &mut self.targets,
                    [w - 165.0, y + if compact { 55.0 } else { 0.0 }, 133.0, 40.0],
                    "Remove",
                    Action::Remove(i),
                    false,
                );
            }
            for (i, (_, name, on)) in self.catalog.native.iter().enumerate() {
                let y = top + (i + self.catalog.installed.entries.len()) as f32 * row_height
                    - self.catalog.scroll;
                self.list_targets.push((
                    [w - 165.0, y, 133.0, if compact { 95.0 } else { 64.0 }],
                    Action::ToggleNative(i),
                ));
                if y < top || y + if compact { 95.0 } else { 64.0 } > bottom {
                    continue;
                }
                ui.text(
                    Face::Medium,
                    19.0,
                    32.0,
                    y,
                    color::TEXT,
                    &truncate(name, 65),
                );
                ui.text(
                    Face::Regular,
                    14.0,
                    32.0,
                    y + 28.0,
                    color::MUTED,
                    "Native Horizon WebAssembly mod / sandboxed",
                );
                toggle(
                    &mut ui,
                    &mut self.targets,
                    [w - 165.0, y + if compact { 55.0 } else { 0.0 }, 133.0, 40.0],
                    *on,
                    Action::ToggleNative(i),
                );
            }
        } else {
            let c = &self.catalog.config;
            ui.text(
                Face::Medium,
                24.0,
                32.0,
                156.0,
                color::TEXT,
                "Launch settings",
            );
            let row_height = if compact { 82.0 } else { 66.0 };
            self.catalog.scroll = self
                .catalog
                .scroll
                .clamp(0.0, (6.0 * row_height - (h - 130.0 - 195.0)).max(0.0));
            self.list_view = Some([
                195.0,
                h - 130.0,
                (6.0 * row_height - (h - 130.0 - 195.0)).max(0.0),
            ]);
            for (i, (title, value, action)) in [
                ("Rendering", c.renderer.label().to_owned(), Action::Renderer),
                (
                    "Client resolution",
                    if self.catalog.editing == Some("render_scale") {
                        self.catalog.render_scale_text.clone()
                    } else {
                        format!("{}%", c.render_scale.percent())
                    },
                    Action::RenderScale,
                ),
                ("Profile", format!("Profile {}", c.profile), Action::Profile),
                (
                    "Orientation",
                    c.orientation.label().to_owned(),
                    Action::Orientation,
                ),
                ("Input", c.input_host.label().to_owned(), Action::InputHost),
                (
                    "Server",
                    if c.server.is_empty() {
                        "Singleplayer / tap to enter server".into()
                    } else {
                        c.server.clone()
                    },
                    Action::Field("server"),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let y = 195.0 + i as f32 * row_height - self.catalog.scroll;
                self.list_targets.push((
                    [
                        if compact { 32.0 } else { w * 0.36 },
                        y,
                        if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                        if compact { 70.0 } else { 46.0 },
                    ],
                    action,
                ));
                if y < 185.0 || y + if compact { 70.0 } else { 46.0 } > h - 130.0 {
                    continue;
                }
                ui.text(
                    Face::Regular,
                    16.0,
                    32.0,
                    y + if compact { 0.0 } else { 13.0 },
                    color::MUTED,
                    title,
                );
                button(
                    &mut ui,
                    &mut self.targets,
                    [
                        if compact { 32.0 } else { w * 0.36 },
                        y + if compact { 24.0 } else { 0.0 },
                        if compact { w - 64.0 } else { w * 0.64 - 32.0 },
                        46.0,
                    ],
                    &format!(
                        "{} {}",
                        value,
                        if matches!(
                            action,
                            Action::Renderer
                                | Action::RenderScale
                                | Action::Profile
                                | Action::Orientation
                                | Action::InputHost
                        ) {
                            "v"
                        } else {
                            ""
                        }
                    ),
                    action,
                    false,
                );
                ui.rounded(
                    32.0,
                    y + if compact { 78.0 } else { 62.0 },
                    w - 64.0,
                    1.0,
                    0.0,
                    color::BORDER,
                );
            }
            button(
                &mut ui,
                &mut self.targets,
                [w * 0.5 + 6.0, h - 114.0, w * 0.5 - 38.0, 42.0],
                "Microsoft / Xbox account",
                Action::Account,
                true,
            );
            button(
                &mut ui,
                &mut self.targets,
                [32.0, h - 114.0, w * 0.5 - 38.0, 42.0],
                "Startup diagnostics",
                Action::Logs,
                false,
            );
        }
        let status = self.error.as_deref().unwrap_or(&self.catalog.status);
        for (i, line) in status
            .chars()
            .collect::<Vec<_>>()
            .chunks(110)
            .take(2)
            .enumerate()
        {
            ui.text(
                Face::Regular,
                14.0,
                32.0,
                h - 47.0 + i as f32 * 18.0,
                color::MUTED,
                &ui.fit_text(
                    Face::Regular,
                    14.0,
                    &line.iter().collect::<String>(),
                    (w - 210.0).max(40.0),
                ),
            );
        }
        button(
            &mut ui,
            &mut self.targets,
            [w - 142.0, h - 52.0, 110.0, 32.0],
            "GitHub",
            Action::Repository,
            false,
        );
        if self.login.visible {
            self.list_targets.clear();
            self.list_view = None;
            self.login.draw(
                &mut ui,
                &mut self.targets,
                &self.catalog.config.name,
                self.catalog.editing == Some("name"),
            );
        }
        if let Some(chooser) = &mut self.chooser {
            chooser.draw(&mut ui, &mut self.targets);
            self.list_targets = chooser.navigation(ui.w);
            self.list_view = Some(chooser.viewport(ui.h));
        }
        if let Some([top, bottom, maximum]) = self.list_view
            && maximum > 0.0
            && bottom > top
        {
            let scroll = self
                .chooser
                .as_ref()
                .map_or(self.catalog.scroll, |c| c.scroll);
            let height = bottom - top;
            let thumb = (height * height / (height + maximum)).max(20.0).min(height);
            let y = top + (height - thumb) * (scroll / maximum).clamp(0.0, 1.0);
            ui.rounded(w - 12.0, top, 4.0, height, 2.0, color::BORDER);
            ui.rounded(w - 12.0, y, 4.0, thumb, 2.0, color::MUTED);
        }
        let caret_visible = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .is_ok_and(|t| t.as_millis() % 1000 < 500);
        if caret_visible && let Some(name) = self.catalog.editing {
            let value = match name {
                "name" => self.catalog.config.name.clone(),
                "server" => self.catalog.config.server.clone(),
                "render_scale" => self.catalog.render_scale_text.clone(),
                _ => format!("Search: {}", self.catalog.query),
            };
            for ([x, y, w, _], action) in &self.targets {
                if *action == Action::Field(name)
                    || (name == "render_scale" && *action == Action::RenderScale)
                {
                    let text = ui.fit_text(Face::Medium, 16.0, &value, w - 24.0);
                    let caret_x = x + 12.0 + ui.measure(Face::Medium, 16.0, &text);
                    ui.rounded(
                        caret_x.min(x + w - 10.0),
                        y + 12.0,
                        1.0,
                        18.0,
                        0.0,
                        color::TEXT,
                    );
                }
            }
        }
        if let Some(action) = self.pending_focus.take() {
            self.focus = self.targets.iter().position(|(_, a)| *a == action);
        }
        if let Some(index) = self.focus
            && let Some(([x, y, w, h], _)) = self.targets.get(index)
        {
            ui.border(*x - 2.0, *y - 2.0, *w + 4.0, *h + 4.0, 2.0, color::TEXT);
            self.cursor = (
                f64::from((x + w / 2.0) * ui.density),
                f64::from((y + h / 2.0) * ui.density),
            );
        }
        ui.finish();

        r.render(&game::loading_frame(&b))?;
        Ok(())
    }
    pub(super) fn activate(&mut self, events: &ActiveEventLoop) -> Result<()> {
        if self.session_worker.is_some() {
            return Ok(());
        }
        let Some(r) = &self.renderer else {
            return Ok(());
        };
        let mut b = HudBatch::new([r.config.width, r.config.height]);
        let ui = Ui::mobile(&self.gui.as_ref().unwrap().atlas, &mut b, (0.0, 0.0));
        let p = self.focus.and_then(|index| self.targets.get(index)).map_or(
            [
                self.cursor.0 as f32 / ui.density,
                self.cursor.1 as f32 / ui.density,
            ],
            |([x, y, w, h], _)| [x + w / 2.0, y + h / 2.0],
        );
        let action = self
            .targets
            .iter()
            .find(|([x, y, w, h], _)| {
                (*x..*x + *w).contains(&p[0]) && (*y..*y + *h).contains(&p[1])
            })
            .map(|(_, a)| *a);
        if self.catalog.busy() {
            return Ok(());
        }
        self.focus = None;
        if action.is_some() && horizon_client::official_install::ready(&self.home) {
            let audio = self.audio.get_or_insert_with(|| {
                horizon_client::audio::Audio::new(&self.home.join(".local/horizon-client/26.3"))
            });
            let settings = horizon_client::settings::Settings::load(
                &self
                    .catalog
                    .config
                    .game_home(&self.home)
                    .join("options.txt"),
            );
            audio.master = settings.volumes[0];
            let mut volumes = settings.volumes;
            volumes[0] = 1.0;
            horizon_client::settings::set_source_volumes(volumes);
            audio.play_button_click();
        }
        match action {
            Some(Action::Repository) => {
                self.catalog.status = if crate::platform::open_repository() {
                    "Repository opened in browser."
                } else {
                    "No browser available to open the repository."
                }
                .into();
            }
            Some(Action::Account) => {
                self.login.visible = true;
            }
            Some(Action::CloseLogin) => {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
            }
            Some(Action::DeviceSignIn) => self
                .login
                .start_device(&self.catalog.config.microsoft_client_id),
            Some(Action::CopyLoginCode) => self.login.copy_code(),
            Some(Action::CancelLogin) => {
                self.login.close();
                self.login.visible = true;
            }
            Some(Action::ReviewApp) => self.login.review_app(),
            Some(Action::SignIn) => self.login.start(&self.catalog.config.microsoft_client_id),
            Some(Action::SignOut) => self.login.sign_out(),
            Some(Action::LoginLink) => self.login.open_link(),
            Some(Action::ToggleNative(i)) => self.catalog.toggle_native(&self.home, i),
            Some(Action::Page(page)) => {
                self.catalog.config.save(&self.home)?;
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.page = page;
                self.catalog.scroll = 0.0;
                self.error = None;
                self.catalog.refresh(&self.home);
                if page.is_catalog() {
                    self.catalog.offset = 0;
                    self.catalog.search(&self.home);
                }
            }
            Some(Action::Play) => self.action(events)?,
            Some(Action::Search) => {
                crate::platform::hide();
                self.catalog.offset = 0;
                self.catalog.search(&self.home);
            }
            Some(Action::FinishNameEdit) => {
                self.catalog.editing = None;
                crate::platform::hide();
                self.catalog.config.save(&self.home)?;
            }
            Some(Action::Field(name)) => {
                if name == "render_scale" {
                    self.catalog.render_scale_text =
                        self.catalog.config.render_scale.percent().to_string();
                    self.chooser = None;
                }
                self.catalog.editing = Some(name);
                let initial = match name {
                    "name" => &self.catalog.config.name,
                    "server" => &self.catalog.config.server,
                    "render_scale" => &self.catalog.render_scale_text,
                    "client_id" => &self.catalog.config.microsoft_client_id,
                    _ => &self.catalog.query,
                };
                crate::platform::show(initial);
            }
            Some(Action::Install(i)) => self.catalog.install(&self.home, i),
            Some(Action::Remove(i)) => self.catalog.remove(&self.home, i),
            Some(Action::Toggle(i)) => self.catalog.toggle(&self.home, i),
            Some(Action::Next) => {
                self.catalog.offset += 12;
                self.catalog.search(&self.home);
            }
            Some(Action::Previous) => {
                self.catalog.offset = self.catalog.offset.saturating_sub(12);
                self.catalog.search(&self.home);
            }
            Some(Action::Profile) => self.open_chooser(chooser::Kind::Profile),
            Some(Action::Renderer) => self.open_chooser(chooser::Kind::Renderer),
            Some(Action::RenderScale) => self.open_chooser(chooser::Kind::RenderScale),
            Some(Action::ChooseRenderScale(scale)) => {
                self.catalog.config.render_scale = scale;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::display::set_scale(scale);
            }
            Some(Action::Orientation) => self.open_chooser(chooser::Kind::Orientation),
            Some(Action::InputHost) => self.open_chooser(chooser::Kind::InputHost),
            Some(Action::ChooseInputHost(mode)) => {
                self.catalog.config.input_host = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                // Host changes occur only when Play is pressed, never during a download/login.
            }
            Some(Action::ChooseOrientation(mode)) => {
                self.catalog.config.orientation = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
                crate::platform::set_orientation(mode);
            }
            Some(Action::ChooseRenderer(mode)) => {
                self.catalog.config.renderer = mode;
                self.catalog.config.save(&self.home)?;
                self.chooser = None;
            }
            Some(Action::ChooseProfile(profile)) => {
                std::fs::create_dir_all(if profile == 0 {
                    self.home.clone()
                } else {
                    self.home
                        .join("profiles")
                        .join(format!("profile-{profile}"))
                })?;
                self.catalog.config.profile = profile;
                self.catalog.config.save(&self.home)?;
                self.catalog.refresh(&self.home);
                self.chooser = None;
            }
            Some(Action::Dismiss) => self.chooser = None,
            Some(Action::Logs) => {
                self.catalog.status = std::fs::read_to_string(self.home.join("android-error.txt"))
                    .unwrap_or_else(|_| {
                        "No startup error. In game: F3 shows device, terrain and frame diagnostics."
                            .into()
                    });
            }
            None => {}
        }
        Ok(())
    }
    pub(super) fn ui_density(&self) -> f32 {
        let (Some(r), Some(gui)) = (&self.renderer, &self.gui) else {
            return 1.0;
        };
        let mut batch = HudBatch::new([r.config.width, r.config.height]);
        Ui::mobile(&gui.atlas, &mut batch, (0.0, 0.0)).density
    }

    pub(super) fn touch_event(&mut self, events: &ActiveEventLoop, t: Touch) -> Result<()> {
        self.focus = None;
        self.pending_focus = None;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        self.cursor = (t.location.x, t.location.y);
        match t.phase {
            TouchPhase::Started => {
                self.drag = Some(super::gesture::Drag::new(t.location.x, t.location.y));
            }
            TouchPhase::Moved => {
                let density = self.ui_density() as f64;
                if let Some(drag) = &mut self.drag {
                    let delta = drag.update(t.location.x, t.location.y, density);
                    if drag.moved {
                        if let Some(chooser) = &mut self.chooser {
                            chooser.scroll = (chooser.scroll + delta).max(0.0);
                        } else if drag.origin_y() / density > 185.0 {
                            self.catalog.scroll = (self.catalog.scroll + delta).max(0.0);
                        }
                    }
                }
            }
            TouchPhase::Ended => {
                if self.drag.take().is_some_and(|drag| !drag.moved) {
                    self.activate(events)?;
                }
            }
            TouchPhase::Cancelled => {
                self.drag = None;
            }
        }
        Ok(())
    }
    pub(super) fn type_key(&mut self, key: KeyEvent) {
        if key.physical_key == PhysicalKey::Code(KeyCode::Escape)
            || key.logical_key
                == winit::keyboard::Key::Named(winit::keyboard::NamedKey::BrowserBack)
        {
            if self.login.visible {
                self.login.hide();
                crate::platform::hide();
                self.catalog.editing = None;
                return;
            }
            if self.chooser.take().is_some() {
                return;
            }
            crate::platform::hide();
            self.catalog.editing = None;
            let _ = self.catalog.config.save(&self.home);
            return;
        }
        if self.catalog.editing.is_none() {
            return;
        }
        let out = match key.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => Some(Output::Backspace),
            PhysicalKey::Code(KeyCode::Enter) => {
                crate::platform::hide();
                Some(Output::Enter)
            }
            _ => key.text.map(|s| Output::Text(s.into())),
        };
        if let Some(out) = out {
            self.catalog.edit(out, &self.home);
        }
    }
}
fn truncate(s: &str, max: usize) -> String {
    let mut value: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        value.push_str("...");
    }
    value
}
