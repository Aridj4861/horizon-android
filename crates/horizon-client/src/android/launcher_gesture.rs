//! Track total drag distance separately from per-event scroll deltas.
pub(super) struct Drag {
    origin: [f64; 2],
    previous_y: f64,
    pub(super) moved: bool,
}

impl Drag {
    pub(super) fn new(x: f64, y: f64) -> Self {
        Self {
            origin: [x, y],
            previous_y: y,
            moved: false,
        }
    }

    pub(super) fn origin_y(&self) -> f64 {
        self.origin[1]
    }

    pub(super) fn update(&mut self, x: f64, y: f64, density: f64) -> f32 {
        let density = density.max(0.01);
        self.moved |= (x - self.origin[0]).hypot(y - self.origin[1]) > 12.0 * density;
        let delta = if self.moved {
            ((self.previous_y - y) / density) as f32
        } else {
            0.0
        };
        self.previous_y = y;
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_vertical_drag_crosses_the_total_threshold() {
        let mut drag = Drag::new(100.0, 300.0);
        for y in [297.0, 294.0, 291.0, 288.0] {
            assert_eq!(drag.update(100.0, y, 1.0), 0.0);
        }
        assert_eq!(drag.update(100.0, 285.0, 1.0), 3.0);
        assert!(drag.moved);
        assert_eq!(drag.origin_y(), 300.0);
        assert_eq!(drag.update(100.0, 290.0, 1.0), -5.0);
    }

    #[test]
    fn density_scales_threshold_and_scroll_together() {
        let mut drag = Drag::new(200.0, 600.0);
        assert_eq!(drag.update(200.0, 580.0, 2.0), 0.0);
        assert_eq!(drag.update(200.0, 570.0, 2.0), 5.0);
        assert!(drag.moved);
    }
}
