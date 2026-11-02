//! Kinematic Mouse Gesture Recognition Engine
//!
//! Provides ultra-low overhead, allocation-free gesture detection specifically tuned for
//! file drag-and-drop activation (Shake gesture and Citrus Arc/Circle gesture).

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Identified gesture types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GestureKind {
    /// Rapid horizontal back-and-forth shake (Dropover / Holdem style)
    Shake,
    /// Swept circular or orbital loop (Sweet Lime citrus slice style)
    Circle,
}

/// A timestamped cursor point in screen coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GesturePoint {
    pub x: f32,
    pub y: f32,
    pub timestamp_ms: u64,
}

/// Kinematic gesture tracker maintaining a rolling window of recent cursor motion
#[derive(Debug, Clone)]
pub struct KinematicGestureTracker {
    points: Vec<GesturePoint>,
    capacity: usize,
    window_ms: u64,
    min_shake_amplitude: f32,
    min_shake_reversals: usize,
    min_circle_angle_rad: f32,
    min_circle_radius: f32,
    allow_shake: bool,
    allow_circle: bool,
    last_trigger_ms: u64,
    debounce_ms: u64,
}

impl Default for KinematicGestureTracker {
    fn default() -> Self {
        Self::new(true, true)
    }
}

impl KinematicGestureTracker {
    /// Creates a new gesture tracker with default thresholds
    pub fn new(allow_shake: bool, allow_circle: bool) -> Self {
        Self {
            points: Vec::with_capacity(64),
            capacity: 64,
            window_ms: 450,
            min_shake_amplitude: 22.0,  // Minimum 22 pixels peak-to-peak
            min_shake_reversals: 3,     // 3 direction flips (left-right-left)
            min_circle_angle_rad: 5.23, // ~300 degrees swept arc
            min_circle_radius: 18.0,    // Minimum 18 px radius
            allow_shake,
            allow_circle,
            last_trigger_ms: 0,
            debounce_ms: 1000, // 1 second cooldown after triggering
        }
    }

    /// Set sensitivity parameters
    pub fn with_sensitivity(mut self, shake_amp: f32, window_ms: u64) -> Self {
        self.min_shake_amplitude = shake_amp;
        self.window_ms = window_ms;
        self
    }

    /// Reset all tracked points and state
    pub fn reset(&mut self) {
        self.points.clear();
    }

    /// Feed a new cursor coordinate into the kinematic tracker.
    /// Returns `Some(GestureKind)` if an intentional activation gesture is detected.
    pub fn feed(&mut self, x: f32, y: f32, timestamp_ms: u64) -> Option<GestureKind> {
        // Enforce trigger debounce cooldown
        if timestamp_ms.saturating_sub(self.last_trigger_ms) < self.debounce_ms {
            return None;
        }

        // Prune points outside current time window
        let cutoff = timestamp_ms.saturating_sub(self.window_ms);
        self.points.retain(|p| p.timestamp_ms >= cutoff);

        // Add new point
        if self.points.len() >= self.capacity {
            self.points.remove(0);
        }
        self.points.push(GesturePoint { x, y, timestamp_ms });

        if self.points.len() < 4 {
            return None;
        }

        // Test Shake
        if self.allow_shake && self.detect_shake() {
            self.last_trigger_ms = timestamp_ms;
            self.points.clear();
            return Some(GestureKind::Shake);
        }

        // Test Circle
        if self.allow_circle && self.detect_circle() {
            self.last_trigger_ms = timestamp_ms;
            self.points.clear();
            return Some(GestureKind::Circle);
        }

        None
    }

    /// Detects horizontal back-and-forth shake gesture
    fn detect_shake(&self) -> bool {
        if self.points.len() < 4 {
            return false;
        }

        let mut reversals = 0usize;
        let mut last_dx: Option<f32> = None;
        let mut last_peak_x = self.points[0].x;
        let mut max_extrema_diff = 0.0f32;

        for window in self.points.windows(2) {
            let dx = window[1].x - window[0].x;
            if dx.abs() < 3.0 {
                continue; // Ignore minor sensor micro-jitter
            }

            if let Some(prev) = last_dx {
                // Direction reversal detected: dx changed sign
                if (dx > 0.0 && prev < 0.0) || (dx < 0.0 && prev > 0.0) {
                    let amp = (window[0].x - last_peak_x).abs();
                    if amp >= self.min_shake_amplitude {
                        reversals += 1;
                        if amp > max_extrema_diff {
                            max_extrema_diff = amp;
                        }
                    }
                    last_peak_x = window[0].x;
                }
            }
            last_dx = Some(dx);
        }

        // Check if the current motion past the last peak meets amplitude
        if let Some(last_pt) = self.points.last() {
            let final_amp = (last_pt.x - last_peak_x).abs();
            if final_amp >= self.min_shake_amplitude && reversals >= self.min_shake_reversals - 1 {
                reversals += 1;
            }
        }

        reversals >= self.min_shake_reversals && max_extrema_diff >= self.min_shake_amplitude
    }

    /// Detects citrus arc or circular loop gesture
    fn detect_circle(&self) -> bool {
        if self.points.len() < 8 {
            return false;
        }

        // Calculate centroid
        let count = self.points.len() as f32;
        let sum_x: f32 = self.points.iter().map(|p| p.x).sum();
        let sum_y: f32 = self.points.iter().map(|p| p.y).sum();
        let cx = sum_x / count;
        let cy = sum_y / count;

        // Verify adequate radius from centroid
        let mut avg_radius = 0.0f32;
        for p in &self.points {
            let r = ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt();
            avg_radius += r;
        }
        avg_radius /= count;

        if avg_radius < self.min_circle_radius {
            return false;
        }

        // Compute accumulated signed angular displacement
        let mut total_angle = 0.0f32;
        let mut prev_theta = (self.points[0].y - cy).atan2(self.points[0].x - cx);

        for p in &self.points[1..] {
            let theta = (p.y - cy).atan2(p.x - cx);
            let mut d_theta = theta - prev_theta;

            // Normalize delta to [-PI, PI] to handle angle wrapping
            while d_theta > PI {
                d_theta -= 2.0 * PI;
            }
            while d_theta < -PI {
                d_theta += 2.0 * PI;
            }

            total_angle += d_theta;
            prev_theta = theta;
        }

        total_angle.abs() >= self.min_circle_angle_rad
    }

    /// Get current tracked point count
    pub fn point_count(&self) -> usize {
        self.points.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shake_gesture_detection() {
        let mut tracker = KinematicGestureTracker::new(true, false);

        // Simulate rapid back-and-forth shake:
        // Left (0) -> Right (40) -> Left (0) -> Right (40) -> Left (0) within 350ms
        let mut time = 1000u64;
        let mut detected = None;

        let x_steps = [
            0.0, 15.0, 30.0, 45.0, // Rightward (+45)
            35.0, 15.0, -5.0, // Leftward (-50)
            10.0, 25.0, 45.0, // Rightward (+50)
            30.0, 10.0, -5.0, // Leftward (-50)
        ];

        for &x in &x_steps {
            time += 25;
            if let Some(kind) = tracker.feed(x, 200.0, time) {
                detected = Some(kind);
                break;
            }
        }

        assert_eq!(detected, Some(GestureKind::Shake));
    }

    #[test]
    fn test_circle_gesture_detection() {
        let mut tracker = KinematicGestureTracker::new(false, true);

        // Simulate smooth circular orbit: R=50 centered at (200, 200)
        let cx = 200.0f32;
        let cy = 200.0f32;
        let r = 50.0f32;
        let mut time = 1000u64;
        let mut detected = None;

        // 16 points tracing 360 degrees (2*PI)
        for i in 0..=16 {
            let angle = (i as f32 / 16.0) * (2.0 * PI);
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            time += 20;

            if let Some(kind) = tracker.feed(x, y, time) {
                detected = Some(kind);
                break;
            }
        }

        assert_eq!(detected, Some(GestureKind::Circle));
    }

    #[test]
    fn test_straight_line_drag_does_not_trigger() {
        let mut tracker = KinematicGestureTracker::new(true, true);

        // Normal straight drag across desktop: 0 to 500 px over 300ms
        let mut time = 1000u64;
        for i in 0..20 {
            let x = (i * 25) as f32;
            let y = (i * 10) as f32;
            time += 15;
            let res = tracker.feed(x, y, time);
            assert!(
                res.is_none(),
                "Straight line drag should not trigger any gesture"
            );
        }
    }

    #[test]
    fn test_micro_jitter_does_not_trigger() {
        let mut tracker = KinematicGestureTracker::new(true, true);

        // Micro-jitter: 1-2px sensor vibration
        let mut time = 1000u64;
        let jitters = [100.0, 101.5, 99.0, 100.5, 101.0, 99.5, 100.0];
        for &x in &jitters {
            time += 30;
            let res = tracker.feed(x, 200.0, time);
            assert!(res.is_none(), "Micro jitter should not trigger gesture");
        }
    }
}
