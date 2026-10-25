use super::*;

pub struct Editor {
    pub selected: Option<Control>,
    option: Option<usize>,
    apply_all: bool,
    original: Settings,
    drag: Option<(u64, Drag)>,
    error: Option<String>,
}
enum Drag {
    Control([f32; 2]),
    Slider(usize),
}
const OPTIONS: [&str; 4] = ["Split", "Left hand", "Auto sprint", "Swap jump/sneak"];

impl TouchControls {
    pub fn open_editor(&mut self) {
        self.clear();
        self.editor = Some(Editor {
            selected: None,
            option: None,
            apply_all: false,
            original: self.settings.clone(),
            drag: None,
            error: None,
        });
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
        if started {
            if layout::contains([0.32, 0.91, 0.11, 0.075], p) {
                match self.settings.save() {
                    Ok(()) => {
                        self.clear();
                        return true;
                    }
                    Err(error) => e.error = Some(error.to_string()),
                }
            } else if layout::contains([0.44, 0.91, 0.11, 0.075], p) {
                self.settings = e.original;
                self.clear();
                return true;
            } else if layout::contains([0.56, 0.91, 0.12, 0.075], p) {
                self.settings.layouts[self.settings.mode.index()] =
                    Settings::default().layouts[self.settings.mode.index()];
            } else if (0.12..0.20).contains(&p[1]) {
                let index = ((p[0] - 0.02) / 0.32).floor() as usize;
                if let Some(mode) = Mode::ALL.get(index) {
                    self.settings.mode = *mode;
                }
            } else if (0.22..0.29).contains(&p[1]) {
                match ((p[0] - 0.02) / 0.24).floor() as usize {
                    0 if self.settings.mode == Mode::Dpad => {
                        self.settings.split_controls = !self.settings.split_controls
                    }
                    1 => self.settings.left_handed = !self.settings.left_handed,
                    2 if self.settings.mode != Mode::Dpad => {
                        self.settings.sprint_with_joystick = !self.settings.sprint_with_joystick
                    }
                    3 if self.settings.mode == Mode::Dpad => {
                        self.settings.swap_jump_sneak = !self.settings.swap_jump_sneak
                    }
                    _ => {}
                }
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
        for (i, mode) in Mode::ALL.into_iter().enumerate() {
            plate(
                b,
                a,
                [0.02 + i as f32 * 0.32, 0.12, 0.30, 0.08],
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
            plate(
                b,
                a,
                [0.02 + i as f32 * 0.24, 0.22, 0.22, 0.07],
                &format!("{name}: {}", if on { "On" } else { "Off" }),
                on,
            );
        }
        let label = "Touch controls: tap a button, then drag it";
        b.text(
            a,
            label,
            (w - HudBatch::text_width(a, label)) / 2.0,
            h * 0.32,
            text,
            true,
        );
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
            ([0.32, 0.91, 0.11, 0.075], "Done"),
            ([0.44, 0.91, 0.11, 0.075], "Cancel"),
            ([0.56, 0.91, 0.12, 0.075], "Reset"),
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
            y * bh + (h * bh - 16.0) / 2.0,
            rgba(35, 35, 35, 255),
            true,
        );
        b.text(
            a,
            second,
            x * bw + 3.0,
            y * bh + (h * bh - 16.0) / 2.0 + 8.0,
            rgba(35, 35, 35, 255),
            true,
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
            true,
        );
    }
}
