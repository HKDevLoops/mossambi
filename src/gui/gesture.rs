//! 2-Time Circular Motion Mouse Gesture Tracker for Mossambi
//! Detects when a user holds down the mouse button (dragging a file) and traces
//! 2 complete circular loops (accumulated rotation >= 720 degrees / ~11.5 radians
//! clockwise or counter-clockwise) within a 2.5-second window.
//! Awakens the Mossambi Sweet Lime Radial Widget centered directly where the user circled.

use std::collections::VecDeque;
use std::f32::consts::PI;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct MouseSample {
    pub x: f32,
    pub y: f32,
    pub timestamp: Instant,
}

pub struct CircleGestureDetector {
    samples: VecDeque<MouseSample>,
    max_history: Duration,
    min_radius: f32,
    max_radius: f32,
    threshold_radians: f32,
    last_trigger: Instant,
    cooldown: Duration,
}

impl Default for CircleGestureDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl CircleGestureDetector {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(160),
            max_history: Duration::from_millis(2600), // 2.6-second sliding window for 2 circles
            min_radius: 15.0,                         // Minimum circle radius (px)
            max_radius: 420.0,                        // Maximum circle radius (px)
            // 2 complete revolutions = 720 degrees (4 * PI ~= 12.566 rad).
            // We use 11.5 rad (~660 degrees) to allow natural human gesture variance while requiring 2 loops.
            threshold_radians: 11.5,
            last_trigger: Instant::now() - Duration::from_secs(10),
            cooldown: Duration::from_millis(1500), // 1.5-second cooldown between gesture triggers
        }
    }

    /// Feeds a new mouse position sample while dragging (left button held).
    /// Returns `Some((center_x, center_y))` if a 2-time circular motion was completed.
    pub fn update(&mut self, x: f32, y: f32, is_dragging: bool) -> Option<(f32, f32)> {
        let now = Instant::now();

        if !is_dragging {
            self.samples.clear();
            return None;
        }

        if now.duration_since(self.last_trigger) < self.cooldown {
            return None;
        }

        // Add new sample
        self.samples.push_back(MouseSample {
            x,
            y,
            timestamp: now,
        });

        // Prune samples older than max_history
        while let Some(front) = self.samples.front() {
            if now.duration_since(front.timestamp) > self.max_history {
                self.samples.pop_front();
            } else {
                break;
            }
        }

        // Need at least 18 samples across time to establish 2 complete revolutions
        if self.samples.len() < 18 {
            return None;
        }

        // Calculate centroid of current samples
        let n = self.samples.len() as f32;
        let cx: f32 = self.samples.iter().map(|s| s.x).sum::<f32>() / n;
        let cy: f32 = self.samples.iter().map(|s| s.y).sum::<f32>() / n;

        // Verify average radius from centroid falls within valid circle bounds
        let mut avg_radius = 0.0f32;
        for s in &self.samples {
            let dx = s.x - cx;
            let dy = s.y - cy;
            avg_radius += (dx * dx + dy * dy).sqrt();
        }
        avg_radius /= n;

        if avg_radius < self.min_radius || avg_radius > self.max_radius {
            return None;
        }

        // Accumulate cumulative angular displacement relative to centroid
        let mut total_angle = 0.0f32;
        let mut prev_angle: Option<f32> = None;

        for s in &self.samples {
            let angle = (s.y - cy).atan2(s.x - cx);
            if let Some(prev) = prev_angle {
                let mut d_theta = angle - prev;
                // Normalize jump across the -PI / +PI branch cut
                while d_theta > PI {
                    d_theta -= 2.0 * PI;
                }
                while d_theta < -PI {
                    d_theta += 2.0 * PI;
                }
                total_angle += d_theta;
            }
            prev_angle = Some(angle);
        }

        // Check if absolute accumulated rotation satisfies 2-time circular motion (>= 720 deg)
        if total_angle.abs() >= self.threshold_radians {
            self.last_trigger = now;
            self.samples.clear();
            return Some((cx, cy));
        }

        None
    }

    pub fn reset(&mut self) {
        self.samples.clear();
    }
}
