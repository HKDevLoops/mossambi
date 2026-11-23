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
            points: Vec::with_capacity(256),
            capacity: 256,
            window_ms: 2600, // 2.6 seconds sliding window to allow 2 complete circles
            min_shake_amplitude: 18.0, // Minimum 18 pixels peak-to-peak
            min_shake_reversals: 3, // 3 direction flips (left-right-left or up-down-up)
            min_circle_angle_rad: 10.5, // 2 complete loops (~600-720 deg)
            min_circle_radius: 14.0, // Minimum 14 px radius
            allow_shake,
            allow_circle,
            last_trigger_ms: 0,
            debounce_ms: 1200, // 1.2 second cooldown after triggering
        }
    }

    /// Set sensitivity parameters
    pub fn with_sensitivity(mut self, shake_amp: f32, window_ms: u64) -> Self {
        self.min_shake_amplitude = shake_amp;
        if window_ms > 0 {
            // Guarantee at least 2200ms window so 2-circle loop has ample time to complete
            self.window_ms = window_ms.max(2200);
        }
        self
    }

    /// Reset all tracked points and state
    pub fn reset(&mut self) {
        self.points.clear();
    }

    /// Feed a new cursor coordinate into the kinematic tracker.
    /// Returns `Some(GestureKind)` if an intentional activation gesture is detected.
    pub fn feed(&mut self, x: f32, y: f32, timestamp_ms: u64) -> Option<GestureKind> {
        // Enforce trigger debounce cooldown after a previous trigger
        if self.last_trigger_ms > 0
            && timestamp_ms.saturating_sub(self.last_trigger_ms) < self.debounce_ms
        {
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

        // Test Shake (Omnidirectional: horizontal or vertical)
        if self.allow_shake && self.detect_shake() {
            self.last_trigger_ms = timestamp_ms;
            self.points.clear();
            return Some(GestureKind::Shake);
        }

        // Test Circle (2 Clockwise or circular revolutions)
        if self.allow_circle && self.detect_circle() {
            self.last_trigger_ms = timestamp_ms;
            self.points.clear();
            return Some(GestureKind::Circle);
        }

        None
    }

    /// Detects omnidirectional back-and-forth shake gesture (horizontal or vertical)
    fn detect_shake(&self) -> bool {
        if self.points.len() < 4 {
            return false;
        }

        // Check horizontal shake
        if self.check_axis_shake(|p| p.x) {
            return true;
        }

        // Check vertical shake
        if self.check_axis_shake(|p| p.y) {
            return true;
        }

        false
    }

    fn check_axis_shake(&self, coord: impl Fn(&GesturePoint) -> f32) -> bool {
        let mut reversals = 0usize;
        let mut last_delta: Option<f32> = None;
        let mut last_peak = coord(&self.points[0]);
        let mut max_extrema_diff = 0.0f32;

        for window in self.points.windows(2) {
            let delta = coord(&window[1]) - coord(&window[0]);
            if delta.abs() < 2.5 {
                continue; // Ignore minor sensor micro-jitter
            }

            if let Some(prev) = last_delta {
                // Direction reversal detected: delta changed sign
                if (delta > 0.0 && prev < 0.0) || (delta < 0.0 && prev > 0.0) {
                    let amp = (coord(&window[0]) - last_peak).abs();
                    if amp >= self.min_shake_amplitude {
                        reversals += 1;
                        if amp > max_extrema_diff {
                            max_extrema_diff = amp;
                        }
                    }
                    last_peak = coord(&window[0]);
                }
            }
            last_delta = Some(delta);
        }

        // Check if current motion past last peak meets amplitude
        if let Some(last_pt) = self.points.last() {
            let final_amp = (coord(last_pt) - last_peak).abs();
            if final_amp >= self.min_shake_amplitude && reversals >= self.min_shake_reversals - 1 {
                reversals += 1;
            }
        }

        reversals >= self.min_shake_reversals && max_extrema_diff >= self.min_shake_amplitude
    }

    /// Detects circular loop gesture (specifically 2 clockwise loops or complete revolutions)
    fn detect_circle(&self) -> bool {
        if self.points.len() < 12 {
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

        if avg_radius < self.min_circle_radius || avg_radius > 450.0 {
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

        // In screen coordinates (+Y down), clockwise rotation gives positive angular displacement.
        // Two full loops: total_angle >= 10.5 rad (~600 to 720 degrees).
        // Also support single sweeping circle or counter-clockwise if threshold allows.
        total_angle.abs() >= self.min_circle_angle_rad || (total_angle >= 10.0)
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
    fn test_vertical_shake_gesture_detection() {
        let mut tracker = KinematicGestureTracker::new(true, false);

        // Simulate rapid vertical back-and-forth shake (up and down)
        let mut time = 1000u64;
        let mut detected = None;

        let y_steps = [
            0.0, 15.0, 30.0, 45.0, // Downward (+45)
            35.0, 15.0, -5.0, // Upward (-50)
            10.0, 25.0, 45.0, // Downward (+50)
            30.0, 10.0, -5.0, // Upward (-50)
        ];

        for &y in &y_steps {
            time += 25;
            if let Some(kind) = tracker.feed(200.0, y, time) {
                detected = Some(kind);
                break;
            }
        }

        assert_eq!(detected, Some(GestureKind::Shake));
    }

    #[test]
    fn test_circle_gesture_detection() {
        let mut tracker = KinematicGestureTracker::new(false, true);

        // Simulate 2 smooth clockwise circular orbits: R=50 centered at (200, 200)
        let cx = 200.0f32;
        let cy = 200.0f32;
        let r = 50.0f32;
        let mut time = 1000u64;
        let mut detected = None;

        // 32 points tracing 2 complete clockwise loops (4*PI)
        for i in 0..=36 {
            let angle = (i as f32 / 16.0) * (2.0 * PI);
            let x = cx + r * angle.cos();
            let y = cy + r * angle.sin();
            time += 25;

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
