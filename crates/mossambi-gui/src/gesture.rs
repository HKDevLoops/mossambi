//! Circle-Drag Mouse Gesture Tracker for Mossambi
//! Detects when a user holds down the mouse button (dragging a file) and traces
//! a circular path (accumulated delta theta >= 300 degrees clockwise or counter-clockwise)
//! to summon the Mossambi radial widget directly under the cursor.

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
            samples: VecDeque::with_capacity(128),
            max_history: Duration::from_millis(1500), // 1.5 second window
            min_radius: 20.0,                         // Minimum circle radius (px)
            max_radius: 350.0,                        // Maximum circle radius (px)
            threshold_radians: (300.0 / 180.0) * PI,  // 300 degrees in radians (~5.24 rad)
            last_trigger: Instant::now() - Duration::from_secs(10),
            cooldown: Duration::from_secs(2),         // 2-second cooldown between gesture triggers
        }
    }

    /// Feeds a new mouse position sample while dragging (left button held).
    /// Returns `true` if a full circle gesture was completed.
    pub fn update(&mut self, x: f32, y: f32, is_dragging: bool) -> bool {
        let now = Instant::now();

        if !is_dragging {
            self.samples.clear();
            return false;
        }

        if now.duration_since(self.last_trigger) < self.cooldown {
            return false;
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

        if self.samples.len() < 12 {
            return false;
        }

        // Calculate centroid of current samples
        let n = self.samples.len() as f32;
        let cx: f32 = self.samples.iter().map(|s| s.x).sum::<f32>() / n;
        let cy: f32 = self.samples.iter().map(|s| s.y).sum::<f32>() / n;

        // Verify radius from centroid falls within valid circle bounds
        let mut avg_radius = 0.0f32;
        for s in &self.samples {
            let dx = s.x - cx;
            let dy = s.y - cy;
            avg_radius += (dx * dx + dy * dy).sqrt();
        }
        avg_radius /= n;

        if avg_radius < self.min_radius || avg_radius > self.max_radius {
            return false;
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

        // Check if absolute accumulated rotation exceeds 300 degrees
        if total_angle.abs() >= self.threshold_radians {
            self.last_trigger = now;
            self.samples.clear();
            return true;
        }

        false
    }

    pub fn reset(&mut self) {
        self.samples.clear();
    }
}
