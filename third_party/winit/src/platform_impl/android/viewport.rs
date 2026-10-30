//! Physical presentation bounds shared by the renderer and absolute input.
use std::sync::Mutex;

static SETTINGS: Mutex<([u32; 4], [u32; 2], [u32; 2])> = Mutex::new(([0; 4], [0; 2], [0; 2]));

pub fn set_insets(insets: [u32; 4]) {
    SETTINGS.lock().unwrap().0 = insets;
}
pub fn set_aspect(aspect: [u32; 2]) {
    SETTINGS.lock().unwrap().1 = aspect;
}
pub fn rect(size: [u32; 2]) -> [u32; 4] {
    let mut state = SETTINGS.lock().unwrap();
    state.2 = size;
    fit(size, state.0, state.1)
}
pub fn origin() -> [u32; 2] {
    let state = SETTINGS.lock().unwrap();
    let [x, y, ..] = fit(state.2, state.0, state.1);
    [x, y]
}

pub fn fit(size: [u32; 2], insets: [u32; 4], aspect: [u32; 2]) -> [u32; 4] {
    let [width, height] = size.map(|v| v.max(1));
    let [left, top, right, bottom] = insets;
    let x = left.min(width - 1);
    let y = top.min(height - 1);
    let w = width.saturating_sub(x).saturating_sub(right).max(1);
    let h = height.saturating_sub(y).saturating_sub(bottom).max(1);
    let [a, b] = aspect;
    if a == 0 || b == 0 {
        return [x, y, w, h];
    }
    let (fw, fh) = if u64::from(w) * u64::from(b) > u64::from(h) * u64::from(a) {
        (
            ((u64::from(h) * u64::from(a) / u64::from(b)) as u32).max(1),
            h,
        )
    } else {
        (
            w,
            ((u64::from(w) * u64::from(b) / u64::from(a)) as u32).max(1),
        )
    };
    [x + (w - fw) / 2, y + (h - fh) / 2, fw, fh]
}

#[cfg(test)]
mod tests {
    use super::fit;
    #[test]
    fn cutout_and_aspect_share_the_same_physical_rect() {
        assert_eq!(fit([1280, 800], [48, 0, 0, 0], [0, 0]), [48, 0, 1232, 800]);
        assert_eq!(fit([1280, 800], [48, 0, 0, 0], [4, 3]), [131, 0, 1066, 800]);
        assert_eq!(fit([1280, 800], [0; 4], [16, 9]), [0, 40, 1280, 720]);
        assert_eq!(fit([800, 1280], [0, 48, 0, 32], [0, 0]), [0, 48, 800, 1200]);
    }
    #[test]
    fn malformed_insets_never_underflow() {
        assert_eq!(fit([0, 0], [u32::MAX; 4], [0, 0]), [0, 0, 1, 1]);
    }
}
