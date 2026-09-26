use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn hover_time() -> f32 {
    if enabled() { HOVER } else { 0.0 }
}

pub const HOVER: f32 = 0.13;

pub const SPEED_PANEL: f32 = 11.0;
pub const SPEED_LIST: f32 = 6.5;
pub const SPEED_INTRO: f32 = 5.5;

#[derive(Clone, Copy)]
pub struct Motion {
    pub enabled: bool,
    pub backdrop: bool,
}

impl Motion {
    pub fn step(&self, current: f32, target: f32, dt: f32, speed: f32) -> f32 {
        if !self.enabled {
            return target;
        }
        let next = current + (target - current) * (1.0 - (-speed * dt).exp());
        if (next - target).abs() < 0.002 { target } else { next }
    }

}

pub fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn back_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let c1 = 1.70158_f32;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

