//! User-added keys share drawing/hit geometry and retain per-finger ownership.
use super::*;
use layout::Placement;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomButton {
    pub key: String,
    pub layouts: [Placement; 3],
}
impl CustomButton {
    pub fn new(key: String, index: usize) -> Self {
        Self {
            key,
            layouts: [Placement::new(
                0.68 + (index % 3) as f32 * 0.1,
                0.35 + (index / 3 % 3) as f32 * 0.12,
                0.105,
            ); 3],
        }
    }
    pub fn label(&self) -> String {
        crate::keybinds::display_name(&self.key).plain()
    }
}
impl TouchControls {
    pub(super) fn fit_placement(&self, mut p: Placement) -> Placement {
        p.size = (p.size * self.settings.button_scale)
            .min(self.aspect())
            .min(1.0);
        let half = [p.size / self.aspect() / 2.0, p.size / 2.0];
        for (i, half) in half.into_iter().enumerate() {
            p.center[i] = p.center[i].clamp(half, 1.0 - half);
        }
        p
    }
    pub(super) fn custom_placement(&self, index: usize) -> Placement {
        let mut p = self.settings.custom_buttons[index].layouts[self.settings.mode.index()];
        if self.settings.left_handed {
            p.center[0] = 1.0 - p.center[0];
        }
        self.fit_placement(p)
    }
    pub fn custom_at(&self, point: [f32; 2]) -> Option<usize> {
        (0..self.settings.custom_buttons.len())
            .rev()
            .find(|&i| layout::contains(self.custom_placement(i).rect(self.aspect()), point))
    }
    pub(super) fn custom_update(
        &mut self,
        id: u64,
        point: [f32; 2],
        started: bool,
        ended: bool,
    ) -> bool {
        if let Some(key) = self.custom_fingers.get(&id).cloned() {
            if ended {
                self.custom_fingers.remove(&id);
                self.fingers.remove(&id);
                if !self.custom_fingers.values().any(|k| *k == key) {
                    self.custom_events.push((key, false));
                }
            }
            return true;
        }
        if started && let Some(index) = self.custom_at(point) {
            let key = self.settings.custom_buttons[index].key.clone();
            let pressed = !self.custom_fingers.values().any(|k| *k == key);
            self.custom_fingers.insert(id, key.clone());
            self.fingers.insert(id, (Role::Ignore, point));
            if pressed {
                self.custom_events.push((key, true));
            }
            self.clicked = true;
            return true;
        }
        false
    }
    pub fn key_events(&mut self) -> Vec<(String, bool)> {
        std::mem::take(&mut self.custom_events)
    }
    pub(super) fn clear_custom(&mut self) {
        let keys: std::collections::BTreeSet<_> = std::mem::take(&mut self.custom_fingers)
            .into_values()
            .collect();
        self.custom_events
            .extend(keys.into_iter().map(|k| (k, false)));
    }
    pub(super) fn draw_custom(&self, b: &mut HudBatch, a: &HudAtlas) {
        for (i, button) in self.settings.custom_buttons.iter().enumerate() {
            let p = self.custom_placement(i);
            let size = p.size * b.height;
            let rect = [
                p.center[0] * b.width - size / 2.0,
                p.center[1] * b.height - size / 2.0,
                size,
                size,
            ];
            let pressed = self.custom_fingers.values().any(|key| *key == button.key);
            let opacity = if self.editor.is_some() {
                p.opacity.max(0.25)
            } else {
                p.opacity
            };
            if opacity <= 0.0 {
                continue;
            }
            let tint = if pressed {
                rgba(170, 210, 150, (opacity * 255.0) as u8)
            } else {
                rgba(255, 255, 255, (opacity * 255.0) as u8)
            };
            nine(b, a, "ui/button_borderless_light", 2.0, rect, tint);
            let label = button.label();
            let width = HudBatch::text_width(a, &label);
            let start = b.vertices.len();
            b.text(
                a,
                &label,
                rect[0] + (size - width) / 2.0,
                rect[1] + (size - 8.0) / 2.0,
                rgba(25, 25, 25, (opacity * 255.0) as u8),
                false,
            );
            let scale = ((size - 6.0).max(1.0) / width.max(1.0))
                .min(((size - 6.0).max(1.0) / 8.0).min(1.0));
            b.scale_since(
                start,
                p.center[0] * b.width,
                p.center[1] * b.height,
                scale,
                scale,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_keys_hold_through_motion_and_shared_keys_release_only_after_last_finger() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.z".into(), 0));
        let p = t.custom_placement(0).center;
        t.update(1, p, true, false);
        t.update(2, p, true, false);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), true)]);
        t.update(1, [0.99, 0.1], false, false);
        assert!(t.key_events().is_empty());
        t.update(1, p, false, true);
        assert!(t.key_events().is_empty());
        t.update(2, p, false, true);
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
    }
    #[test]
    fn cancellation_releases_all_custom_keys_and_scaling_preserves_saved_sizes() {
        let mut t = TouchControls::default();
        t.viewport(1920, 1080);
        t.settings
            .custom_buttons
            .push(CustomButton::new("key.keyboard.z".into(), 0));
        let stored = t.settings.custom_buttons[0].layouts[0].size;
        t.settings.button_scale = 2.0;
        assert_eq!(t.custom_placement(0).size, stored * 2.0);
        t.update(1, t.custom_placement(0).center, true, false);
        t.key_events();
        t.clear();
        assert_eq!(t.key_events(), [("key.keyboard.z".into(), false)]);
        assert_eq!(t.settings.custom_buttons[0].layouts[0].size, stored);
    }
}
