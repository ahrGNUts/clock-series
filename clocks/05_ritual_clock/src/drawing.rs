//! Drawing module for the Ritual Clock
//!
//! Handles rendering of nodes, animations, trails, and the digital overlay.

use std::time::Instant;

use nannou::prelude::*;
use shared::{DstChange, TimeData};

use crate::stage::StageGeometry;
use crate::Model;

/// Color palette for the ritual clock theme
#[allow(dead_code)]
pub mod colors {
    use nannou::prelude::*;

    /// Deep background
    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 15,
        green: 18,
        blue: 25,
        standard: std::marker::PhantomData,
    };

    /// Hour node base color
    pub const HOUR_NODE: Srgb<u8> = Srgb {
        red: 80,
        green: 100,
        blue: 140,
        standard: std::marker::PhantomData,
    };

    /// Hour node active/shimmer color
    pub const HOUR_NODE_ACTIVE: Srgb<u8> = Srgb {
        red: 180,
        green: 200,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Hour node highlight color
    pub const HOUR_NODE_HIGHLIGHT: Srgb<u8> = Srgb {
        red: 255,
        green: 220,
        blue: 100,
        standard: std::marker::PhantomData,
    };

    /// Beat node base color
    pub const BEAT_NODE: Srgb<u8> = Srgb {
        red: 60,
        green: 70,
        blue: 90,
        standard: std::marker::PhantomData,
    };

    /// Beat node pulse color
    pub const BEAT_NODE_PULSE: Srgb<u8> = Srgb {
        red: 150,
        green: 180,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Trail color
    pub const TRAIL: Srgb<u8> = Srgb {
        red: 100,
        green: 150,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Overlay background (use overlay_bg() function)
    pub fn overlay_bg() -> Srgba<u8> {
        srgba(20, 25, 35, 230)
    }

    /// Primary text
    pub const TEXT_PRIMARY: Srgb<u8> = Srgb {
        red: 220,
        green: 225,
        blue: 235,
        standard: std::marker::PhantomData,
    };

    /// Secondary text
    pub const TEXT_SECONDARY: Srgb<u8> = Srgb {
        red: 140,
        green: 150,
        blue: 170,
        standard: std::marker::PhantomData,
    };

    /// DST warning color
    pub const DST_WARNING: Srgb<u8> = Srgb {
        red: 255,
        green: 150,
        blue: 80,
        standard: std::marker::PhantomData,
    };

    /// Ghost beat color (use ghost_beat() function)
    pub fn ghost_beat() -> Srgba<u8> {
        srgba(255, 150, 80, 100)
    }

    /// Focus ring color
    pub const FOCUS_RING: Srgb<u8> = Srgb {
        red: 100,
        green: 180,
        blue: 255,
        standard: std::marker::PhantomData,
    };
}

/// Draw the entire stage (nodes, trails, animations)
pub fn draw_stage(draw: &Draw, geometry: &StageGeometry, model: &Model) {
    let now = Instant::now();

    // Calculate retune rotation if active
    let retune_rotation = calculate_retune_rotation(model, now);

    // Draw beat nodes (outer ring)
    draw_beat_nodes(draw, geometry, model, now, retune_rotation);

    // Draw hour nodes (inner ring)
    draw_hour_nodes(draw, geometry, model, now);

    // Draw gesture trails
    if model.should_draw_trails() {
        draw_trails(draw, geometry, model, now);
    }

    // Draw DST ghost beat if upcoming
    if matches!(model.time_data.dst_change, DstChange::Upcoming { .. }) {
        draw_ghost_beat(draw, geometry, model, now);
    }
}

/// Calculate retune rotation angle
fn calculate_retune_rotation(model: &Model, now: Instant) -> f32 {
    if let Some(start) = model.retune_start {
        let elapsed = now.duration_since(start).as_secs_f32();
        let duration = 0.3; // 300ms

        if elapsed < duration {
            // Rotation: Δ = (newOffsetMinutes - oldOffsetMinutes) * 0.05°
            let delta_deg = model.retune_delta_offset as f32 * 0.05;
            let progress = elapsed / duration;
            // Ease out
            let eased = 1.0 - (1.0 - progress).powi(2);
            return delta_deg * eased * std::f32::consts::PI / 180.0;
        }
    }
    0.0
}

/// Draw beat nodes with pulse animation
fn draw_beat_nodes(
    draw: &Draw,
    geometry: &StageGeometry,
    model: &Model,
    now: Instant,
    retune_rotation: f32,
) {
    for j in 0..60 {
        let (bx, by) = geometry.beat_positions[j];

        // Apply retune rotation around center
        let (bx, by) = if retune_rotation != 0.0 {
            let dx = bx - geometry.cx;
            let dy = by - geometry.cy;
            let cos_r = retune_rotation.cos();
            let sin_r = retune_rotation.sin();
            (
                geometry.cx + dx * cos_r - dy * sin_r,
                geometry.cy + dx * sin_r + dy * cos_r,
            )
        } else {
            (bx, by)
        };

        // Calculate pulse animation
        let (scale, color, ring_outline) = calculate_beat_pulse(model, j, now);

        let radius = geometry.beat_node_radius * scale;

        // Draw the node
        draw.ellipse()
            .x_y(bx, by)
            .radius(radius)
            .color(color);

        // Draw ring outline for reduced motion pulse
        if ring_outline {
            draw.ellipse()
                .x_y(bx, by)
                .radius(radius + 3.0)
                .no_fill()
                .stroke(colors::BEAT_NODE_PULSE)
                .stroke_weight(2.0);
        }
    }
}

/// Calculate beat pulse scale, color, and ring outline flag
/// Returns (scale, color, ring_outline)
fn calculate_beat_pulse(model: &Model, beat_index: usize, now: Instant) -> (f32, Srgb<u8>, bool) {
    if let Some(start) = model.beat_pulse_start {
        if beat_index == model.beat_pulse_index {
            let elapsed_ms = now.duration_since(start).as_secs_f32() * 1000.0;

            if elapsed_ms < 360.0 {
                if model.reduced_motion {
                    // Reduced motion: ring outline for 200ms (no scaling)
                    if elapsed_ms < 200.0 {
                        return (1.0, colors::BEAT_NODE, true); // Ring outline enabled
                    }
                } else {
                    // Normal animation
                    let scale = if elapsed_ms < 120.0 {
                        // Phase 1: 0-120ms, scale 1.0 → 1.8
                        let t = elapsed_ms / 120.0;
                        1.0 + 0.8 * t
                    } else {
                        // Phase 2: 120-360ms, scale 1.8 → 1.0 ease-out
                        let t = (elapsed_ms - 120.0) / 240.0;
                        let eased = 1.0 - (1.0 - t).powi(2);
                        1.8 - 0.8 * eased
                    };

                    // Interpolate color
                    let color_t = 1.0 - (elapsed_ms / 360.0);
                    let r = lerp_u8(colors::BEAT_NODE.red, colors::BEAT_NODE_PULSE.red, color_t);
                    let g = lerp_u8(colors::BEAT_NODE.green, colors::BEAT_NODE_PULSE.green, color_t);
                    let b = lerp_u8(colors::BEAT_NODE.blue, colors::BEAT_NODE_PULSE.blue, color_t);

                    return (scale, Srgb::new(r, g, b), false);
                }
            }
        }
    }

    (1.0, colors::BEAT_NODE, false)
}

/// Draw hour nodes with shimmer animation
fn draw_hour_nodes(draw: &Draw, geometry: &StageGeometry, model: &Model, now: Instant) {
    // Calculate minuteIntensity = minute / 59 for gradual buildup
    let minute_intensity = model.time_data.minute as f32 / 59.0;
    let current_hour_index = (model.time_data.hour12 % 12) as usize;

    for i in 0..12 {
        let (hx, hy) = geometry.hour_positions[i];

        // Base color with minute intensity applied to current hour
        let mut color = if i == current_hour_index {
            // Gradually brighten current hour as minutes progress
            let r = lerp_u8(colors::HOUR_NODE.red, colors::HOUR_NODE_ACTIVE.red, minute_intensity * 0.3);
            let g = lerp_u8(colors::HOUR_NODE.green, colors::HOUR_NODE_ACTIVE.green, minute_intensity * 0.3);
            let b = lerp_u8(colors::HOUR_NODE.blue, colors::HOUR_NODE_ACTIVE.blue, minute_intensity * 0.3);
            Srgb::new(r, g, b)
        } else {
            colors::HOUR_NODE
        };
        let mut extra_alpha: Option<f32> = None;

        // Check if this hour is highlighted
        if model.highlighted_hour == Some(i) {
            color = colors::HOUR_NODE_HIGHLIGHT;
        }

        // Calculate shimmer animation
        if let Some(start) = model.hour_shimmer_start {
            if i == model.hour_shimmer_index {
                let elapsed_ms = now.duration_since(start).as_secs_f32() * 1000.0;

                if elapsed_ms < 600.0 {
                    if model.reduced_motion {
                        // Reduced motion: static highlight for 400ms
                        if elapsed_ms < 400.0 {
                            color = colors::HOUR_NODE_ACTIVE;
                        }
                    } else {
                        // Normal: opacity 0 → 1 → 0 ease-in-out
                        let t = elapsed_ms / 600.0;
                        let alpha = if t < 0.5 {
                            // Ease in
                            2.0 * t * t
                        } else {
                            // Ease out
                            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                        };
                        // We'll draw an overlay with this alpha
                        extra_alpha = Some(1.0 - alpha);
                        color = colors::HOUR_NODE_ACTIVE;
                    }
                }
            }
        }

        // Check for DST echo effect (fall-back)
        if matches!(model.time_data.dst_change, DstChange::JustOccurred { delta_minutes, .. } if delta_minutes < 0)
        {
            // Draw echo effect - duplicate shimmer on current hour
            if i == (model.time_data.hour12 % 12) as usize {
                let echo_alpha = (model.animation_time * 2.0).sin().abs() * 0.3;
                draw.ellipse()
                    .x_y(hx, hy)
                    .radius(geometry.hour_node_radius * 1.3)
                    .color(srgba(
                        colors::HOUR_NODE_ACTIVE.red,
                        colors::HOUR_NODE_ACTIVE.green,
                        colors::HOUR_NODE_ACTIVE.blue,
                        (echo_alpha * 255.0) as u8,
                    ));
            }
        }

        // Draw the node
        draw.ellipse()
            .x_y(hx, hy)
            .radius(geometry.hour_node_radius)
            .color(color);

        // Draw shimmer overlay if animating
        if let Some(alpha) = extra_alpha {
            draw.ellipse()
                .x_y(hx, hy)
                .radius(geometry.hour_node_radius * 1.2)
                .color(srgba(
                    colors::HOUR_NODE_ACTIVE.red,
                    colors::HOUR_NODE_ACTIVE.green,
                    colors::HOUR_NODE_ACTIVE.blue,
                    ((1.0 - alpha) * 150.0) as u8,
                ));
        }

        // Draw focus ring if this is the highlighted hour
        if model.highlighted_hour == Some(i) {
            draw.ellipse()
                .x_y(hx, hy)
                .radius(geometry.hour_node_radius + 4.0)
                .no_fill()
                .stroke(colors::FOCUS_RING)
                .stroke_weight(2.0);
        }
    }
}

/// Draw gesture trails
fn draw_trails(draw: &Draw, geometry: &StageGeometry, model: &Model, now: Instant) {
    if model.trail_points.len() < 2 {
        return;
    }

    let lifetime = 2.0; // seconds
    let base_width = geometry.trail_base_width() * (0.5 + model.gesture_sensitivity);

    // Draw trail segments
    for i in 1..model.trail_points.len() {
        let p0 = &model.trail_points[i - 1];
        let p1 = &model.trail_points[i];

        // Calculate alpha based on age
        let age = now.duration_since(p1.instant).as_secs_f32();
        let alpha_raw = (1.0 - age / lifetime).clamp(0.0, 1.0);
        let alpha = alpha_raw * alpha_raw; // squared falloff

        if alpha < 0.01 {
            continue;
        }

        let width = base_width * alpha;

        draw.line()
            .start(pt2(p0.x, p0.y))
            .end(pt2(p1.x, p1.y))
            .stroke_weight(width)
            .color(srgba(
                colors::TRAIL.red,
                colors::TRAIL.green,
                colors::TRAIL.blue,
                (alpha * 200.0) as u8,
            ));
    }
}

/// Draw ghost beat for DST warning
fn draw_ghost_beat(draw: &Draw, geometry: &StageGeometry, model: &Model, _now: Instant) {
    if model.reduced_motion {
        // Static badge instead of animation - draw indicator near center
        draw.text("DST")
            .x_y(geometry.cx, geometry.cy - geometry.r_hour - 30.0)
            .color(colors::DST_WARNING)
            .font_size(14);
        return;
    }

    // Faint pulsing overlay on all beat nodes
    let pulse = (model.animation_time * 3.0).sin() * 0.5 + 0.5;
    let alpha = (pulse * 80.0) as u8;

    for j in 0..60 {
        let (bx, by) = geometry.beat_positions[j];
        draw.ellipse()
            .x_y(bx, by)
            .radius(geometry.beat_node_radius * 1.5)
            .color(srgba(
                colors::DST_WARNING.red,
                colors::DST_WARNING.green,
                colors::DST_WARNING.blue,
                alpha,
            ));
    }
}

/// Draw digital overlay
pub fn draw_overlay(
    draw: &Draw,
    geometry: &StageGeometry,
    time_data: &TimeData,
    highlighted_hour: Option<usize>,
    always_on: bool,
) {
    let overlay_width = 200.0;
    let overlay_height = 80.0;
    let overlay_x = geometry.cx;
    let overlay_y = geometry.cy;

    // Draw background
    draw.rect()
        .x_y(overlay_x, overlay_y)
        .w_h(overlay_width, overlay_height)
        .color(colors::overlay_bg());

    // Draw time
    let time_str = format!(
        "{:02}:{:02}:{:02} {}",
        time_data.hour12, time_data.minute, time_data.second, time_data.meridiem
    );
    draw.text(&time_str)
        .x_y(overlay_x, overlay_y + 15.0)
        .color(colors::TEXT_PRIMARY)
        .font_size(24)
        .w(overlay_width - 20.0);

    // Draw highlighted hour or date
    let secondary_text = if let Some(hour) = highlighted_hour {
        format!("Hour {} highlighted", hour_to_display(hour))
    } else {
        time_data.format_date()
    };
    draw.text(&secondary_text)
        .x_y(overlay_x, overlay_y - 20.0)
        .color(colors::TEXT_SECONDARY)
        .font_size(12)
        .w(overlay_width - 20.0);

    // Draw always-on indicator
    if always_on {
        draw.text("●")
            .x_y(overlay_x + overlay_width / 2.0 - 15.0, overlay_y + overlay_height / 2.0 - 10.0)
            .color(colors::TEXT_SECONDARY)
            .font_size(8);
    }
}

/// Convert hour index to display format (1-12)
fn hour_to_display(hour_index: usize) -> u32 {
    if hour_index == 0 {
        12
    } else {
        hour_index as u32
    }
}

/// Linear interpolation for u8 values
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * t.clamp(0.0, 1.0)) as u8
}

/// Draw error banner for TZ data issues
pub fn draw_error_banner(draw: &Draw, window_rect: Rect) {
    let banner_height = 40.0;
    let banner_y = window_rect.top() - 60.0;

    // Background
    draw.rect()
        .x_y(0.0, banner_y)
        .w_h(window_rect.w(), banner_height)
        .color(srgba(120u8, 40u8, 40u8, 220u8));

    // Text
    draw.text("⚠ Timezone data may be missing or stale. Showing UTC as fallback.")
        .x_y(0.0, banner_y)
        .color(colors::TEXT_PRIMARY)
        .font_size(14)
        .w(window_rect.w() - 40.0);
}

/// Draw toast notifications
pub fn draw_toasts(draw: &Draw, window_rect: Rect, toasts: &[crate::Toast]) {
    let toast_width = 300.0;
    let toast_height = 40.0;
    let padding = 10.0;
    let start_y = window_rect.bottom() + 150.0; // Above conductor panel

    for (i, toast) in toasts.iter().enumerate() {
        let y = start_y + (i as f32) * (toast_height + padding);
        let alpha = (toast.alpha() * 220.0) as u8;

        // Background
        draw.rect()
            .x_y(0.0, y)
            .w_h(toast_width, toast_height)
            .color(srgba(40u8, 45u8, 55u8, alpha));

        // Border
        draw.rect()
            .x_y(0.0, y)
            .w_h(toast_width, toast_height)
            .no_fill()
            .stroke(srgba(80u8, 90u8, 110u8, alpha))
            .stroke_weight(1.0);

        // Text
        let text_alpha = (toast.alpha() * 255.0) as u8;
        draw.text(&toast.message)
            .x_y(0.0, y)
            .color(srgba(220u8, 225u8, 235u8, text_alpha))
            .font_size(12)
            .w(toast_width - 20.0);
    }
}

