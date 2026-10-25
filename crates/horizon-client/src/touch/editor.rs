use super::*;

pub struct Editor {
    pub selected: Option<Control>,
    settings_page: bool,
    option: Option<usize>,
    apply_all: bool,
    original: Settings,
    drag: Option<(u64, Drag)>,
    error: Option<String>,
}
impl Editor {
    pub(super) fn settings_page(&self) -> bool {
        self.settings_page
    }
}

enum Drag {
    Control([f32; 2]),
    Slider(usize),
    Sensitivity,
}
const OPTIONS: [&str; 4] = ["Split", "Left hand", "Auto sprint", "Swap jump/sneak"];

impl TouchControls {
    pub fn open_editor(&mut self) {
        self.clear();
        self.editor = Some(Editor {
            selected: None,
            settings_page: false,
            option: None,
            apply_all: false,
            original: self.settings.clone(),
            drag: None,
            error: None,
        });
    }
    pub fn open_settings(&mut self) {
        self.open_editor();
        self.editor.as_mut().unwrap().settings_page = true;
    }
    pub fn cancel_editor(&mut self) {
        if let Some(e) = self.editor.take() {
            self.settings = e.original;
        }
        self.clear();
    }
    /// The editor owns every finger: editing cannot move, mine or open menus.
    /// Returns true when Done/Cancel closed the editor.
    pub fn edit_touch(&mut self, id: u64, p: [f32; 2], started: bool, ended: bool) -> bool {
        let Some(mut e) = self.editor.take() else {
            return false;
        };
        if e.settings_page {
            if started {
                if layout::contains([0.05, 0.84, 0.28, 0.10], p) {
                    e.settings_page = false;
                } else if layout::contains([0.36, 0.84, 0.28, 0.10], p) {
                    match self.settings.save() {
                        Ok(()) => {
                            self.clear();
                            return true;
                        }
                        Err(error) => e.error = Some(error.to_string()),
                    }
                } else if layout::contains([0.67, 0.84, 0.28, 0.10], p) {
                    self.settings = e.original;
                    self.clear();
                    return true;
                } else if layout::contains([0.52, 0.70, 0.43, 0.08], p) {
                    e.drag = Some((id, Drag::Sensitivity));
                } else {
                    for (i, mode) in Mode::ALL.into_iter().enumerate() {
                        if layout::contains([0.05, 0.19 + i as f32 * 0.16, 0.43, 0.13], p) {
                            self.settings.mode = mode;
                        }
                    }
                    for i in 0..4 {
                        if layout::contains([0.52, 0.19 + i as f32 * 0.12, 0.43, 0.09], p) {
                            match i {
                                0 if self.settings.mode == Mode::Dpad => {
                                    self.settings.split_controls ^= true
                                }
                                1 => self.settings.left_handed ^= true,
                                2 if self.settings.mode != Mode::Dpad => {
                                    self.settings.sprint_with_joystick ^= true
                                }
                                3 if self.settings.mode == Mode::Dpad => {
                                    self.settings.swap_jump_sneak ^= true
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            if matches!(&e.drag, Some((owner, Drag::Sensitivity)) if *owner == id) {
                self.settings.sensitivity = 0.1 + ((p[0] - 0.53) / 0.41).clamp(0.0, 1.0) * 2.9;
                if ended {
                    e.drag = None;
                }
            }
            self.editor = Some(e);
            return false;
        }
        if started {
            if layout::contains([0.02, 0.14, 0.335, 0.075], p) {
                e.settings_page = true;
                e.drag = None;
                self.editor = Some(e);
                return false;
            }
            if layout::contains([0.02, 0.04, 0.105, 0.075], p) {
                match self.settings.save() {
                    Ok(()) => {
                        self.clear();
                        return true;
                    }
                    Err(error) => e.error = Some(error.to_string()),
                }
            } else if layout::contains([0.135, 0.04, 0.105, 0.075], p) {
                self.settings = e.original;
                self.clear();
                return true;
            } else if layout::contains([0.25, 0.04, 0.105, 0.075], p) {
                self.settings.layouts[self.settings.mode.index()] =
                    Settings::default().layouts[self.settings.mode.index()];
            } else if layout::contains([0.32, 0.61, 0.36, 0.06], p)
                && e.selected.is_some()
                && e.option.is_some()
            {
                e.apply_all = !e.apply_all;
            } else if layout::contains([0.32, 0.68, 0.36, 0.07], p)
                && e.selected.is_some()
                && e.option.is_some()
            {
                e.drag = Some((id, Drag::Slider(e.option.unwrap())));
            } else if layout::contains([0.32, 0.79, 0.17, 0.07], p) && e.selected.is_some() {
                e.option = Some(1);
            } else if layout::contains([0.51, 0.79, 0.17, 0.07], p) && e.selected.is_some() {
                e.option = Some(0);
            } else if let Some(c) = Control::ALL.into_iter().rev().find(|c| {
                self.visible(*c)
                    && layout::contains(self.settings.placement(*c).rect(self.aspect()), p)
            }) {
                e.selected = Some(c);
                e.option = None;
                let center = self.settings.placement(c).center;
                e.drag = Some((id, Drag::Control([p[0] - center[0], p[1] - center[1]])));
            } else {
                e.selected = None;
                e.option = None;
            }
        }
        if let Some((owner, drag)) = &e.drag
            && *owner == id
            && let Some(selected) = e.selected
        {
            let aspect = self.aspect();
            let slot = self.settings.slot(selected);
            let place = &mut self.settings.layouts[self.settings.mode.index()][slot];
            match drag {
                Drag::Control(offset) => {
                    let mut center = [p[0] - offset[0], p[1] - offset[1]];
                    if self.settings.left_handed {
                        center[0] = 1.0 - center[0];
                    }
                    place.center = [
                        center[0].clamp(place.size / aspect / 2.0, 1.0 - place.size / aspect / 2.0),
                        center[1].clamp(place.size / 2.0, 1.0 - place.size / 2.0),
                    ];
                }
                Drag::Sensitivity => {}
                Drag::Slider(index) => {
                    let value = ((p[0] - 0.335) / 0.33).clamp(0.0, 1.0);
                    for (i, place) in self.settings.layouts[self.settings.mode.index()]
                        .iter_mut()
                        .enumerate()
                    {
                        if i == slot || e.apply_all {
                            if *index == 0 {
                                place.size = 0.045 + value * 0.405;
                            } else {
                                place.opacity = value;
                            }
                        }
                    }
                }
            }
            if ended {
                e.drag = None;
            }
        }

        self.editor = Some(e);
        false
    }
    pub(super) fn draw_editor(&self, b: &mut HudBatch, a: &HudAtlas) {
        let e = self.editor.as_ref().unwrap();
        let (w, h) = (b.width, b.height);
        let text = rgba(245, 245, 245, 255);
        if e.settings_page {
            b.fill(a, 0.0, 0.0, w, h, rgba(24, 26, 29, 255));
            let title = "Touch controls";
            b.text(
                a,
                title,
                (w - HudBatch::text_width(a, title)) / 2.0,
                h * 0.07,
                text,
                true,
            );
            for (i, mode) in Mode::ALL.into_iter().enumerate() {
                plate(
                    b,
                    a,
                    [0.05, 0.19 + i as f32 * 0.16, 0.43, 0.13],
                    mode.label(),
                    self.settings.mode == mode,
                );
            }
            for (i, (name, on)) in OPTIONS
                .into_iter()
                .zip([
                    self.settings.split_controls,
                    self.settings.left_handed,
                    self.settings.sprint_with_joystick,
                    self.settings.swap_jump_sneak,
                ])
                .enumerate()
            {
                if (i == 0 || i == 3) && self.settings.mode != Mode::Dpad
                    || i == 2 && self.settings.mode == Mode::Dpad
                {
                    continue;
                }
                let y = 0.19 + i as f32 * 0.12;
                b.text(a, name, w * 0.52, h * (y + 0.025), text, true);
                // A sliding switch keeps the setting state visible at a glance.
                b.fill(
                    a,
                    w * 0.87,
                    h * y,
                    w * 0.08,
                    h * 0.08,
                    if on {
                        rgba(65, 145, 55, 255)
                    } else {
                        rgba(80, 80, 80, 255)
                    },
                );
                b.fill(
                    a,
                    w * if on { 0.912 } else { 0.872 },
                    h * (y + 0.008),
                    w * 0.036,
                    h * 0.064,
                    rgba(235, 235, 235, 255),
                );
            }
            b.text(
                a,
                &format!("Sensitivity: {:.2}", self.settings.sensitivity),
                w * 0.52,
                h * 0.65,
                text,
                true,
            );
            nine(
                b,
                a,
                "ui/slider_background",
                1.0,
                [w * 0.53, h * 0.73, w * 0.41, h * 0.025],
                rgba(255, 255, 255, 255),
            );
            nine(
                b,
                a,
                "ui/slider_button_default",
                2.0,
                [
                    w * (0.53 + (self.settings.sensitivity - 0.1) / 2.9 * 0.41) - 3.0,
                    h * 0.71,
                    6.0,
                    h * 0.065,
                ],
                rgba(255, 255, 255, 255),
            );
            for (x, label) in [(0.05, "Customize layout"), (0.36, "Save"), (0.67, "Cancel")] {
                plate(b, a, [x, 0.84, 0.28, 0.10], label, false);
            }
            if e.error.is_some() {
                b.text(
                    a,
                    "Save failed. Retry Save.",
                    w * 0.05,
                    h * 0.77,
                    rgba(255, 120, 120, 255),
                    true,
                );
            }
            return;
        }
        plate(b, a, [0.02, 0.14, 0.335, 0.075], "Touch settings...", false);
        if let Some(selected) = e.selected {
            let p = self.settings.placement(selected);
            plate(
                b,
                a,
                [0.32, 0.79, 0.17, 0.07],
                "Opacity",
                e.option == Some(1),
            );
            plate(b, a, [0.51, 0.79, 0.17, 0.07], "Size", e.option == Some(0));
            if let Some(option) = e.option {
                b.fill(
                    a,
                    w * 0.31,
                    h * 0.59,
                    w * 0.38,
                    h * 0.18,
                    rgba(15, 15, 15, 235),
                );
                plate(
                    b,
                    a,
                    [0.32, 0.61, 0.36, 0.06],
                    if e.apply_all {
                        "Apply to all: On"
                    } else {
                        "Apply to all: Off"
                    },
                    e.apply_all,
                );
                let value = if option == 0 {
                    (p.size - 0.045) / 0.405
                } else {
                    p.opacity
                };
                let percent = if option == 0 {
                    p.size * 100.0
                } else {
                    p.opacity * 100.0
                };
                b.text(
                    a,
                    &format!("{} {:.0}%", selected.name(), percent),
                    w * 0.325,
                    h * 0.68,
                    text,
                    true,
                );
                nine(
                    b,
                    a,
                    "ui/slider_background",
                    1.0,
                    [w * 0.335, h * 0.72, w * 0.33, h * 0.025],
                    rgba(255, 255, 255, 255),
                );
                nine(
                    b,
                    a,
                    "ui/slider_button_default",
                    2.0,
                    [w * (0.335 + value * 0.33) - 3.0, h * 0.707, 6.0, h * 0.05],
                    rgba(255, 255, 255, 255),
                );
            }
        }
        for (rect, label) in [
            ([0.02, 0.04, 0.105, 0.075], "Done"),
            ([0.135, 0.04, 0.105, 0.075], "Cancel"),
            ([0.25, 0.04, 0.105, 0.075], "Reset"),
        ] {
            plate(b, a, rect, label, false);
        }
        if e.error.is_some() {
            b.text(
                a,
                "Save failed. Retry Done.",
                w * 0.33,
                h * 0.88,
                rgba(255, 120, 120, 255),
                true,
            );
        }
    }
}
fn plate(b: &mut HudBatch, a: &HudAtlas, [x, y, w, h]: [f32; 4], label: &str, on: bool) {
    let (bw, bh) = (b.width, b.height);
    nine(
        b,
        a,
        "ui/button_borderless_light",
        1.0,
        [x * bw, y * bh, w * bw, h * bh],
        if on {
            rgba(100, 210, 90, 255)
        } else {
            rgba(255, 255, 255, 255)
        },
    );
    if let Some((first, second)) = label.split_once(" & ") {
        b.text(
            a,
            &format!("{first} &"),
            x * bw + 3.0,
            y * bh + (h * bh - 20.0) / 2.0,
            rgba(35, 35, 35, 255),
            false,
        );
        b.text(
            a,
            second,
            x * bw + 3.0,
            y * bh + (h * bh - 20.0) / 2.0 + 10.0,
            rgba(35, 35, 35, 255),
            false,
        );
    } else {
        let mut display = label.to_owned();
        while HudBatch::text_width(a, &display) > w * bw - 6.0 && !display.is_empty() {
            display.pop();
        }
        b.text(
            a,
            &display,
            x * bw + 3.0,
            y * bh + (h * bh - 8.0) / 2.0,
            rgba(35, 35, 35, 255),
            false,
        );
    }
}
