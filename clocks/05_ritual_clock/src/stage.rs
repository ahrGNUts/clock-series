//! Stage geometry calculations for the Ritual Clock
//!
//! Handles positioning of hour "chorus" nodes and beat nodes in concentric circles,
//! as well as hit testing for interaction.

use nannou::prelude::*;
use std::f32::consts::PI;

/// Stage geometry with all calculated positions
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StageGeometry {
    /// Center X position
    pub cx: f32,
    /// Center Y position
    pub cy: f32,
    /// Stage size (min of width, height - controls)
    pub stage_size: f32,
    /// Radius for hour nodes
    pub r_hour: f32,
    /// Radius for beat nodes
    pub r_beat: f32,
    /// Radius for labels (optional)
    pub r_label: f32,
    /// Hour node radius (for drawing)
    pub hour_node_radius: f32,
    /// Beat node radius (for drawing)
    pub beat_node_radius: f32,
    /// Positions of the 12 hour nodes
    pub hour_positions: [(f32, f32); 12],
    /// Positions of the 60 beat nodes
    pub beat_positions: [(f32, f32); 60],
}

impl StageGeometry {
    /// Calculate geometry from window dimensions
    ///
    /// Per spec: stageSize = min(containerWidth, containerHeight - controlsHeight)
    pub fn calculate(window_rect: Rect, controls_height: f32) -> Self {
        let available_width = window_rect.w();
        let available_height = window_rect.h() - controls_height - 60.0; // Account for title

        let stage_size = available_width.min(available_height);

        // Center position (accounting for bottom panel)
        let cx = window_rect.x();
        let cy = window_rect.y() + controls_height / 2.0;

        // Radii per spec
        let r_hour = 0.24 * stage_size;
        let r_beat = 0.38 * stage_size;
        let r_label = 0.46 * stage_size;

        // Node sizes per spec
        let hour_node_radius = 0.028 * stage_size;
        let beat_node_radius = 0.012 * stage_size;

        // Calculate hour positions (12 nodes, 30° apart)
        // Angle 0 at top: θ0 = 90° = π/2 (in standard math coords with y-up)
        // Clockwise means subtracting angle as index increases
        let theta_0 = PI / 2.0;
        let mut hour_positions = [(0.0f32, 0.0f32); 12];
        for i in 0..12 {
            // Clockwise: subtract angle (negative direction in standard coords)
            let theta = theta_0 - (i as f32) * (30.0 * PI / 180.0);
            hour_positions[i] = (cx + r_hour * theta.cos(), cy + r_hour * theta.sin());
        }

        // Calculate beat positions (60 nodes, 6° apart)
        let mut beat_positions = [(0.0f32, 0.0f32); 60];
        for j in 0..60 {
            // Clockwise: subtract angle
            let theta = theta_0 - (j as f32) * (6.0 * PI / 180.0);
            beat_positions[j] = (cx + r_beat * theta.cos(), cy + r_beat * theta.sin());
        }

        Self {
            cx,
            cy,
            stage_size,
            r_hour,
            r_beat,
            r_label,
            hour_node_radius,
            beat_node_radius,
            hour_positions,
            beat_positions,
        }
    }

    /// Hit test for hour nodes
    ///
    /// Returns the index of the hour node if hit, None otherwise.
    /// Uses a slightly larger hit area for accessibility (~40px minimum).
    pub fn hit_test_hour_node(&self, x: f32, y: f32) -> Option<usize> {
        // Minimum hit radius for accessibility
        let min_hit_radius = 20.0; // ~40px diameter
        let hit_radius = self.hour_node_radius.max(min_hit_radius);

        for (i, &(hx, hy)) in self.hour_positions.iter().enumerate() {
            let dx = x - hx;
            let dy = y - hy;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= hit_radius * hit_radius {
                return Some(i);
            }
        }
        None
    }

    /// Hit test for beat nodes
    ///
    /// Returns the index of the beat node if hit, None otherwise.
    #[allow(dead_code)]
    pub fn hit_test_beat_node(&self, x: f32, y: f32) -> Option<usize> {
        // Minimum hit radius for accessibility
        let min_hit_radius = 10.0;
        let hit_radius = self.beat_node_radius.max(min_hit_radius);

        for (j, &(bx, by)) in self.beat_positions.iter().enumerate() {
            let dx = x - bx;
            let dy = y - by;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= hit_radius * hit_radius {
                return Some(j);
            }
        }
        None
    }

    /// Get the angle for a beat node index (for retune animation)
    #[allow(dead_code)]
    pub fn beat_angle(&self, index: usize) -> f32 {
        let theta_0 = PI / 2.0;
        theta_0 - (index as f32) * (6.0 * PI / 180.0)
    }

    /// Get trail base width per spec
    pub fn trail_base_width(&self) -> f32 {
        0.010 * self.stage_size
    }
}

