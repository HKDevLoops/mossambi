//! Motion primitives for the Mossambi UI.
//!
//! Every animated surface in the app (hover, selection, peek, toolbar, widget,
//! scroll) can be tuned independently from Settings via [`AnimStyle`]. The
//! default is a critically-damped spring, which is the motion model that best
//! matches modern desktop UX (Windows 11 / macOS Sonoma) because it never
//! overshoots noticeably yet still feels physically alive.
//!
//! The integrator is a semi-implicit Euler step with sub-stepping so that a
//! dropped frame (long delta) cannot make the spring explode.

use crate::core::config::{AnimStyle, AnimationSettings};
use eframe::egui;

/// A single damped-spring value.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
    /// Spring constant. Higher = snappier.
    pub stiffness: f32,
    /// Damping ratio multiplier. 1.0 = critical damping.
    pub damping: f32,
    /// Virtual mass for momentum calculation.
    pub mass: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            value: 0.0,
            velocity: 0.0,
            target: 0.0,
            stiffness: 240.0,
            damping: 1.0,
            mass: 1.0,
        }
    }
}

impl Spring {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            target: value,
            ..Default::default()
        }
    }

    /// Snap immediately to `target` with no motion.
    pub fn set(&mut self, target: f32) {
        self.value = target;
        self.target = target;
        self.velocity = 0.0;
    }

    /// Advance the simulation by `dt` seconds and return the new value.
    pub fn step(&mut self, dt: f32) -> f32 {
        self.step_with_params(dt, self.stiffness, self.damping, self.mass)
    }

    /// Advance the simulation using explicit physical parameters.
    pub fn step_with_params(&mut self, dt: f32, stiffness: f32, damping: f32, mass: f32) -> f32 {
        let dt = dt.clamp(0.0, 0.1);
        let mass = mass.max(0.01);
        // Sub-step so large frame gaps stay stable.
        let steps = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 24);
        let h = dt / steps as f32;
        for _ in 0..steps {
            let delta = self.value - self.target;
            let accel = (-stiffness * delta
                - 2.0 * damping * (stiffness * mass).sqrt() * self.velocity)
                / mass;
            self.velocity += accel * h;
            self.value += self.velocity * h;
        }
        if (self.value - self.target).abs() < 0.0005 && self.velocity.abs() < 0.005 {
            self.value = self.target;
            self.velocity = 0.0;
        }
        self.value
    }

    pub fn at_rest(&self) -> bool {
        (self.value - self.target).abs() < 0.001 && self.velocity.abs() < 0.01
    }
}

/// The frame delta used by the app's animation clock, in seconds.
pub fn frame_dt(ctx: &egui::Context) -> f32 {
    ctx.input(|i| i.stable_dt).clamp(0.0, 0.1)
}

/// Advance a spring according to the user-selected motion style.
///
/// * [`AnimStyle::Spring`]    - critically-damped physical spring.
/// * [`AnimStyle::Snappy`]    - fast exponential ease-out (150ms).
/// * [`AnimStyle::Natural`]   - Apple HIG standard ease-out (250ms).
/// * [`AnimStyle::Bouncy`]    - dynamic physical spring with overshoot.
/// * [`AnimStyle::Gentle`]    - smooth fluid Apple HIG ease-in-out (350ms).
/// * [`AnimStyle::Cinematic`] - dramatic 600ms slow-motion ease.
/// * [`AnimStyle::Fade`]      - linear ramp.
/// * [`AnimStyle::None`]      - instant snap.
pub fn advance(spring: &mut Spring, target: f32, style: AnimStyle, dt: f32) -> f32 {
    spring.target = target;
    match style {
        AnimStyle::Spring => spring.step(dt),
        AnimStyle::Snappy => {
            let k = 1.0 - (-24.0 * dt).exp();
            spring.value += (target - spring.value) * k;
            if (spring.value - target).abs() < 0.001 {
                spring.value = target;
            }
            spring.velocity = 0.0;
            spring.value
        }
        AnimStyle::Natural => {
            let k = 1.0 - (-14.0 * dt).exp();
            spring.value += (target - spring.value) * k;
            if (spring.value - target).abs() < 0.001 {
                spring.value = target;
            }
            spring.velocity = 0.0;
            spring.value
        }
        AnimStyle::Bouncy => {
            // Under-damped spring (0.55 damping ratio) produces authentic bounce
            spring.step_with_params(dt, spring.stiffness, 0.55, spring.mass)
        }
        AnimStyle::Gentle => {
            let k = 1.0 - (-8.0 * dt).exp();
            spring.value += (target - spring.value) * k;
            if (spring.value - target).abs() < 0.001 {
                spring.value = target;
            }
            spring.velocity = 0.0;
            spring.value
        }
        AnimStyle::Cinematic => {
            let k = 1.0 - (-4.5 * dt).exp();
            spring.value += (target - spring.value) * k;
            if (spring.value - target).abs() < 0.001 {
                spring.value = target;
            }
            spring.velocity = 0.0;
            spring.value
        }
        AnimStyle::Fade => {
            let k = dt * 9.0;
            spring.value += (target - spring.value) * k.clamp(0.0, 1.0);
            if (spring.value - target).abs() < 0.002 {
                spring.value = target;
            }
            spring.velocity = 0.0;
            spring.value
        }
        AnimStyle::None => {
            spring.set(target);
            spring.value
        }
    }
}

/// Advance a spring using full global animation settings.
pub fn advance_with_settings(
    spring: &mut Spring,
    target: f32,
    style: AnimStyle,
    dt: f32,
    settings: &AnimationSettings,
) -> f32 {
    if settings.reduced_motion {
        spring.set(target);
        return spring.value;
    }
    spring.stiffness = settings.spring_stiffness;
    spring.damping = settings.spring_damping;
    spring.mass = settings.spring_mass;
    advance(spring, target, style, dt)
}

/// A registry of independent animated scalars keyed by a stable id.
///
/// Used for per-row / per-tile hover fades (checkbox reveal, icon lift) where
/// each item needs its own spring but the total number of live items is small.
#[derive(Debug, Default)]
pub struct AnimRegistry {
    springs: std::collections::HashMap<u64, Spring>,
}

impl AnimRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get (creating if needed) the spring for `id`.
    pub fn spring(&mut self, id: u64) -> &mut Spring {
        self.springs.entry(id).or_default()
    }

    /// Drive the spring for `id` toward `target` and return the eased value.
    pub fn drive(&mut self, id: u64, target: f32, style: AnimStyle, dt: f32) -> f32 {
        let spring = self.springs.entry(id).or_default();
        advance(spring, target, style, dt)
    }

    /// Drop springs that have settled at their target, keeping the map small.
    pub fn prune(&mut self) {
        self.springs.retain(|_, s| !s.at_rest());
    }

    pub fn clear(&mut self) {
        self.springs.clear();
    }
}

/// Convenience: a stable 64-bit id for a path, used to key hover animations.
pub fn path_id(path: &std::path::Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    h.finish()
}
