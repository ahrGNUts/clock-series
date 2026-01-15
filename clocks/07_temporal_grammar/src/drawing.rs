//! Drawing module - Render functions for the Temporal Grammar clock
//!
//! Handles rendering of all visual layers, overlays, and UI elements
//! using nannou's Draw API.

use nannou::prelude::*;
use shared::{DstChange, TimeData};

use crate::geometry::{DstKnot, GeometryParams, PhaseRing};

/// Color palette for the temporal grammar aesthetic
pub mod colors {
    use nannou::prelude::*;

    /// Deep background - near black with subtle warmth
    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 12,
        green: 10,
        blue: 18,
        standard: std::marker::PhantomData,
    };

    /// Foundation layer (hour polygon) - deep indigo
    pub const FOUNDATION: Srgb<u8> = Srgb {
        red: 45,
        green: 40,
        blue: 90,
        standard: std::marker::PhantomData,
    };

    /// Foundation stroke
    pub const FOUNDATION_STROKE: Srgb<u8> = Srgb {
        red: 80,
        green: 70,
        blue: 140,
        standard: std::marker::PhantomData,
    };

    /// Tension layer (minute superellipse) - warm amber
    pub const TENSION: Srgb<u8> = Srgb {
        red: 180,
        green: 120,
        blue: 60,
        standard: std::marker::PhantomData,
    };

    /// Tension stroke
    pub const TENSION_STROKE: Srgb<u8> = Srgb {
        red: 220,
        green: 160,
        blue: 80,
        standard: std::marker::PhantomData,
    };

    /// Phase ring marks - cool cyan
    pub const PHASE_MARK: Srgb<u8> = Srgb {
        red: 60,
        green: 140,
        blue: 160,
        standard: std::marker::PhantomData,
    };

    /// Phase ring highlighted mark
    pub const PHASE_HIGHLIGHT: Srgb<u8> = Srgb {
        red: 100,
        green: 240,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Phase needle
    pub const PHASE_NEEDLE: Srgb<u8> = Srgb {
        red: 255,
        green: 255,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// DST knot - warning orange
    pub const DST_KNOT: Srgb<u8> = Srgb {
        red: 255,
        green: 140,
        blue: 60,
        standard: std::marker::PhantomData,
    };

    /// DST knot (just occurred) - cool blue
    pub const DST_KNOT_PAST: Srgb<u8> = Srgb {
        red: 100,
        green: 180,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Truth anchor overlay background
    pub fn overlay_bg() -> Srgba<u8> {
        srgba(20, 18, 28, 240)
    }

    /// Primary text
    pub const TEXT_PRIMARY: Srgb<u8> = Srgb {
        red: 240,
        green: 240,
        blue: 245,
        standard: std::marker::PhantomData,
    };

    /// Secondary text
    pub const TEXT_SECONDARY: Srgb<u8> = Srgb {
        red: 140,
        green: 140,
        blue: 150,
        standard: std::marker::PhantomData,
    };

    /// Decode mode guides
    pub const DECODE_GUIDE: Srgb<u8> = Srgb {
        red: 120,
        green: 200,
        blue: 120,
        standard: std::marker::PhantomData,
    };

    /// Decode mode labels
    pub const DECODE_LABEL: Srgb<u8> = Srgb {
        red: 180,
        green: 255,
        blue: 180,
        standard: std::marker::PhantomData,
    };

    /// Focus ring
    pub const FOCUS_RING: Srgb<u8> = Srgb {
        red: 100,
        green: 200,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// HUD accent
    pub const HUD_ACCENT: Srgb<u8> = Srgb {
        red: 100,
        green: 200,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Help panel background
    pub fn help_bg() -> Srgba<u8> {
        srgba(25, 22, 35, 250)
    }
}

/// Draw the foundation layer (hour polygon)
pub fn draw_foundation_layer(draw: &Draw, points: &[Point2]) {
    if points.is_empty() {
        return;
    }

    // Fill
    draw.polygon()
        .points(points.iter().cloned())
        .color(srgba(
            colors::FOUNDATION.red,
            colors::FOUNDATION.green,
            colors::FOUNDATION.blue,
            180,
        ));

    // Stroke
    let mut stroke_points: Vec<Point2> = points.to_vec();
    stroke_points.push(points[0]); // Close the polygon

    draw.polyline()
        .weight(2.0)
        .points(stroke_points)
        .color(colors::FOUNDATION_STROKE);
}

/// Draw the tension layer (minute superellipse)
pub fn draw_tension_layer(draw: &Draw, points: &[Point2]) {
    if points.is_empty() {
        return;
    }

    // Fill with lower opacity
    draw.polygon()
        .points(points.iter().cloned())
        .color(srgba(
            colors::TENSION.red,
            colors::TENSION.green,
            colors::TENSION.blue,
            100,
        ));

    // Stroke
    let mut stroke_points: Vec<Point2> = points.to_vec();
    stroke_points.push(points[0]);

    draw.polyline()
        .weight(2.5)
        .points(stroke_points)
        .color(colors::TENSION_STROKE);
}

/// Draw the phase layer (second ring with marks and needle)
pub fn draw_phase_layer(draw: &Draw, ring: &PhaseRing, view_zoom: f32) {
    let mark_size = 4.0 * view_zoom;
    let highlight_size = 8.0 * view_zoom;

    // Draw all marks
    for (i, mark) in ring.marks.iter().enumerate() {
        let is_highlighted = i == ring.highlighted_index;
        let is_major = i % 5 == 0;

        if is_highlighted {
            // Highlighted mark - larger and brighter
            draw.ellipse()
                .xy(*mark)
                .radius(highlight_size)
                .color(colors::PHASE_HIGHLIGHT);

            // Glow effect
            draw.ellipse()
                .xy(*mark)
                .radius(highlight_size * 1.5)
                .color(srgba(
                    colors::PHASE_HIGHLIGHT.red,
                    colors::PHASE_HIGHLIGHT.green,
                    colors::PHASE_HIGHLIGHT.blue,
                    60,
                ));
        } else if is_major {
            // Major marks (every 5 seconds)
            draw.ellipse()
                .xy(*mark)
                .radius(mark_size * 1.2)
                .color(colors::PHASE_MARK);
        } else {
            // Minor marks
            draw.ellipse()
                .xy(*mark)
                .radius(mark_size * 0.6)
                .color(srgba(
                    colors::PHASE_MARK.red,
                    colors::PHASE_MARK.green,
                    colors::PHASE_MARK.blue,
                    150,
                ));
        }
    }

    // Draw needle from center to current position
    let needle_end = pt2(
        ring.center.x + ring.radius * 0.85 * ring.needle_angle.cos(),
        ring.center.y + ring.radius * 0.85 * ring.needle_angle.sin(),
    );

    draw.line()
        .start(ring.center)
        .end(needle_end)
        .weight(2.0)
        .color(colors::PHASE_NEEDLE);

    // Needle tip
    draw.ellipse()
        .xy(needle_end)
        .radius(4.0 * view_zoom)
        .color(colors::PHASE_NEEDLE);
}

/// Draw the DST knot
pub fn draw_dst_knot(draw: &Draw, knot: &DstKnot) {
    let color = if knot.is_upcoming {
        colors::DST_KNOT
    } else {
        colors::DST_KNOT_PAST
    };

    // Draw the bezier loop as connected curves
    // First lobe
    draw.line()
        .start(knot.anchor)
        .end(knot.control_points[0])
        .weight(2.0)
        .color(color);

    draw.line()
        .start(knot.control_points[0])
        .end(knot.control_points[1])
        .weight(2.5)
        .color(color);

    draw.line()
        .start(knot.control_points[1])
        .end(knot.control_points[2])
        .weight(2.0)
        .color(color);

    // Second lobe
    draw.line()
        .start(knot.control_points[2])
        .end(knot.control_points[3])
        .weight(2.5)
        .color(color);

    draw.line()
        .start(knot.control_points[3])
        .end(knot.anchor)
        .weight(2.0)
        .color(color);

    // Draw anchor point
    draw.ellipse()
        .xy(knot.anchor)
        .radius(5.0)
        .color(color);

    // Glow around the knot
    for cp in &knot.control_points {
        draw.ellipse()
            .xy(*cp)
            .radius(knot.amplitude * 0.3)
            .color(srgba(color.red, color.green, color.blue, 40));
    }
}

/// Draw the Truth Anchor overlay
pub fn draw_truth_anchor_overlay(
    draw: &Draw,
    time_data: &TimeData,
    position: Point2,
    tz_name: &str,
) {
    let overlay_width = 320.0;
    let overlay_height = 140.0;
    let padding = 15.0;

    // Background
    draw.rect()
        .xy(position)
        .w_h(overlay_width, overlay_height)
        .color(colors::overlay_bg());

    // Border
    draw.rect()
        .xy(position)
        .w_h(overlay_width, overlay_height)
        .no_fill()
        .stroke(colors::HUD_ACCENT)
        .stroke_weight(2.0);

    // Time (large)
    let time_str = format!(
        "{:02}:{:02}:{:02} {}",
        time_data.hour12, time_data.minute, time_data.second, time_data.meridiem
    );
    draw.text(&time_str)
        .xy(position + vec2(0.0, overlay_height / 2.0 - 30.0))
        .color(colors::TEXT_PRIMARY)
        .font_size(28)
        .w(overlay_width - padding * 2.0);

    // Date
    let date_str = time_data.format_date();
    draw.text(&date_str)
        .xy(position + vec2(0.0, overlay_height / 2.0 - 60.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(14)
        .w(overlay_width - padding * 2.0);

    // Timezone info
    let dst_str = if time_data.is_dst { "DST On" } else { "DST Off" };
    let tz_str = format!(
        "{} · {} · {}",
        tz_name,
        time_data.format_utc_offset(),
        dst_str
    );
    draw.text(&tz_str)
        .xy(position + vec2(0.0, overlay_height / 2.0 - 85.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(12)
        .w(overlay_width - padding * 2.0);

    // DST change warning if applicable
    match &time_data.dst_change {
        DstChange::Upcoming { instant, delta_minutes } => {
            let hours_until = (*instant - chrono::Utc::now()).num_hours();
            let direction = if *delta_minutes > 0 {
                "spring forward"
            } else {
                "fall back"
            };
            let warning = format!("⚠ DST change in {}h ({})", hours_until, direction);
            draw.text(&warning)
                .xy(position + vec2(0.0, overlay_height / 2.0 - 110.0))
                .color(colors::DST_KNOT)
                .font_size(11)
                .w(overlay_width - padding * 2.0);
        }
        DstChange::JustOccurred { delta_minutes, .. } => {
            let direction = if *delta_minutes > 0 {
                "sprang forward"
            } else {
                "fell back"
            };
            let info = format!("ℹ Clocks {} recently", direction);
            draw.text(&info)
                .xy(position + vec2(0.0, overlay_height / 2.0 - 110.0))
                .color(colors::DST_KNOT_PAST)
                .font_size(11)
                .w(overlay_width - padding * 2.0);
        }
        DstChange::None => {}
    }
}

/// Draw Decode Mode guides and labels
pub fn draw_decode_mode_guides(
    draw: &Draw,
    params: &GeometryParams,
    ring: &PhaseRing,
    center: Point2,
    canvas_rect: Rect,
) {
    // Position labels relative to canvas edges with padding
    let padding = 15.0;
    let left_x = canvas_rect.left() + padding;
    let right_x = canvas_rect.right() - padding - 120.0; // Leave room for label width

    // Hour polygon label (top-left)
    let hour_label = format!("V = 3 + {} = {}", params.hour, params.vertex_count);
    draw.text(&hour_label)
        .xy(pt2(left_x + 80.0, canvas_rect.top() - 60.0))
        .color(colors::DECODE_LABEL)
        .font_size(14)
        .w(180.0)
        .left_justify();

    // Superellipse exponent label (right side, upper)
    let exp_label = format!("e = {:.2}", params.exponent);
    draw.text(&exp_label)
        .xy(pt2(right_x, center.y + 80.0))
        .color(colors::DECODE_LABEL)
        .font_size(14)
        .w(120.0)
        .left_justify();

    // Minute rotation label (right side, middle-upper)
    let rot_label = format!("rot = {:.1}°", params.minute_rotation_deg);
    draw.text(&rot_label)
        .xy(pt2(right_x, center.y + 50.0))
        .color(colors::DECODE_LABEL)
        .font_size(14)
        .w(120.0)
        .left_justify();

    // Phase angle label (bottom-left)
    let phase_label = format!("φ = {:.1}°", params.phase_deg);
    draw.text(&phase_label)
        .xy(pt2(left_x + 80.0, canvas_rect.bottom() + 80.0))
        .color(colors::DECODE_LABEL)
        .font_size(14)
        .w(120.0)
        .left_justify();

    // TZ transform labels (right side, lower)
    let tz_rot_label = format!("tzRot = {:.1}°", params.tz_rotation_deg);
    draw.text(&tz_rot_label)
        .xy(pt2(right_x, center.y - 50.0))
        .color(colors::DECODE_LABEL)
        .font_size(14)
        .w(130.0)
        .left_justify();

    let tz_skew_label = format!("tzSkewX = {:.3}", params.tz_skew_x);
    draw.text(&tz_skew_label)
        .xy(pt2(right_x, center.y - 80.0))
        .color(colors::DECODE_LABEL)
        .font_size(14)
        .w(130.0)
        .left_justify();

    // Draw guide line from center to highlighted second mark
    if ring.highlighted_index < ring.marks.len() {
        let target = ring.marks[ring.highlighted_index];
        draw.line()
            .start(center)
            .end(target)
            .weight(1.0)
            .color(srgba(
                colors::DECODE_GUIDE.red,
                colors::DECODE_GUIDE.green,
                colors::DECODE_GUIDE.blue,
                150,
            ));
    }

    // Draw crosshairs at center
    let crosshair_size = 20.0;
    draw.line()
        .start(center + vec2(-crosshair_size, 0.0))
        .end(center + vec2(crosshair_size, 0.0))
        .weight(1.0)
        .color(colors::DECODE_GUIDE);
    draw.line()
        .start(center + vec2(0.0, -crosshair_size))
        .end(center + vec2(0.0, crosshair_size))
        .weight(1.0)
        .color(colors::DECODE_GUIDE);
}

/// Draw Explicit Mode (standard time readout replacing canvas)
pub fn draw_explicit_mode(draw: &Draw, time_data: &TimeData, rect: Rect, tz_name: &str) {
    let center = rect.xy();

    // Large time display
    let time_str = format!(
        "{:02}:{:02}:{:02}",
        time_data.hour12, time_data.minute, time_data.second
    );
    draw.text(&time_str)
        .xy(center + vec2(0.0, 60.0))
        .color(colors::TEXT_PRIMARY)
        .font_size(72)
        .w(rect.w());

    // AM/PM
    draw.text(&time_data.meridiem.to_string())
        .xy(center + vec2(180.0, 75.0))
        .color(colors::HUD_ACCENT)
        .font_size(28)
        .w(100.0);

    // Date
    let date_str = time_data.format_date();
    draw.text(&date_str)
        .xy(center + vec2(0.0, 0.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(24)
        .w(rect.w());

    // Timezone info
    let dst_str = if time_data.is_dst { "DST Active" } else { "Standard Time" };
    let tz_str = format!("{} · {} · {}", tz_name, time_data.format_utc_offset(), dst_str);
    draw.text(&tz_str)
        .xy(center + vec2(0.0, -50.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(18)
        .w(rect.w());

    // DST change info
    match &time_data.dst_change {
        DstChange::Upcoming { instant, delta_minutes } => {
            let hours_until = (*instant - chrono::Utc::now()).num_hours();
            let direction = if *delta_minutes > 0 {
                "spring forward"
            } else {
                "fall back"
            };
            let warning = format!("DST change in {} hours ({})", hours_until, direction);
            draw.text(&warning)
                .xy(center + vec2(0.0, -100.0))
                .color(colors::DST_KNOT)
                .font_size(16)
                .w(rect.w());
        }
        DstChange::JustOccurred { delta_minutes, .. } => {
            let direction = if *delta_minutes > 0 {
                "sprang forward"
            } else {
                "fell back"
            };
            let info = format!("Clocks {} recently", direction);
            draw.text(&info)
                .xy(center + vec2(0.0, -100.0))
                .color(colors::DST_KNOT_PAST)
                .font_size(16)
                .w(rect.w());
        }
        DstChange::None => {}
    }

    // Mode indicator
    draw.text("EXPLICIT MODE")
        .xy(center + vec2(0.0, -150.0))
        .color(colors::HUD_ACCENT)
        .font_size(12)
        .w(rect.w());
}

/// Draw the "How to Read This Clock" help panel
pub fn draw_help_panel(draw: &Draw, canvas_rect: Rect) {
    let panel_width = 500.0;
    let panel_height = 400.0;
    let center = canvas_rect.xy();

    // Background
    draw.rect()
        .xy(center)
        .w_h(panel_width, panel_height)
        .color(colors::help_bg());

    // Border
    draw.rect()
        .xy(center)
        .w_h(panel_width, panel_height)
        .no_fill()
        .stroke(colors::HUD_ACCENT)
        .stroke_weight(2.0);

    // Title
    draw.text("How to Read This Clock")
        .xy(center + vec2(0.0, panel_height / 2.0 - 30.0))
        .color(colors::TEXT_PRIMARY)
        .font_size(22)
        .w(panel_width - 40.0);

    // Content - each item is (is_header, text)
    let content = [
        (true, "HOUR (Inner Polygon)"),
        (false, "  Number of sides = 3 + hour"),
        (false, "  4 sides = 1:00, 15 sides = 12:00"),
        (true, "MINUTE (Middle Shape)"),
        (false, "  Shape morphs from round to square"),
        (false, "  Round (e≈1.2) = :00, Square (e≈4) = :59"),
        (false, "  Rotates 360° clockwise through the hour"),
        (true, "SECOND (Outer Ring)"),
        (false, "  60 marks around the ring"),
        (false, "  Highlighted mark = current second"),
        (true, "TIMEZONE"),
        (false, "  Offset rotates/skews all layers"),
        (false, "  DST adds extra rotation + shear"),
        (true, "DST KNOT"),
        (false, "  Appears 24h before DST change"),
        (false, "  Grows larger as transition approaches"),
    ];

    let line_height = 20.0;
    let start_y = panel_height / 2.0 - 70.0;
    let left_edge = center.x - panel_width / 2.0 + 25.0;

    for (i, (is_header, text)) in content.iter().enumerate() {
        let y = center.y + start_y - (i as f32) * line_height;

        let color = if *is_header {
            colors::HUD_ACCENT
        } else {
            colors::TEXT_SECONDARY
        };

        let font_size = if *is_header { 13 } else { 12 };

        draw.text(text)
            .xy(pt2(left_edge + (panel_width - 50.0) / 2.0, y))
            .color(color)
            .font_size(font_size)
            .w(panel_width - 50.0)
            .left_justify();
    }

    // Close hint
    draw.text("Press ? or Escape to close")
        .xy(center + vec2(0.0, -panel_height / 2.0 + 20.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(11)
        .w(panel_width);
}

/// Draw minimal HUD elements (TZ icon hint, DST dot, Truth Anchor hint)
pub fn draw_hud(
    draw: &Draw,
    window_rect: Rect,
    is_dst: bool,
    dst_change: &DstChange,
    truth_anchor_hint: bool,
) {
    let margin = 20.0;

    // TZ icon hint (top-left)
    draw.text("Z: Timezone")
        .xy(pt2(
            window_rect.left() + 60.0,
            window_rect.top() - margin,
        ))
        .color(colors::TEXT_SECONDARY)
        .font_size(11)
        .w(100.0)
        .left_justify();

    // DST status label (top-left, next to TZ)
    let dst_label_pos = pt2(window_rect.left() + 160.0, window_rect.top() - margin);
    let (dst_label, dst_color) = match dst_change {
        DstChange::Upcoming { .. } => ("DST Soon", colors::DST_KNOT),
        DstChange::JustOccurred { .. } => ("DST Changed", colors::DST_KNOT_PAST),
        DstChange::None => {
            if is_dst {
                ("DST", colors::DST_KNOT)
            } else {
                ("Standard", colors::TEXT_SECONDARY)
            }
        }
    };
    draw.text(dst_label)
        .xy(dst_label_pos)
        .color(dst_color)
        .font_size(11)
        .w(80.0)
        .left_justify();

    // Truth Anchor hint (top-right)
    if truth_anchor_hint {
        draw.text("Hold Space: Reveal Time")
            .xy(pt2(
                window_rect.right() - 100.0,
                window_rect.top() - margin,
            ))
            .color(colors::TEXT_SECONDARY)
            .font_size(11)
            .w(180.0)
            .right_justify();
    }

    // Keyboard shortcuts hint (bottom)
    draw.text("D: Decode  |  ?: Help  |  Tab: Focus")
        .xy(pt2(window_rect.x(), window_rect.bottom() + margin))
        .color(srgba(
            colors::TEXT_SECONDARY.red,
            colors::TEXT_SECONDARY.green,
            colors::TEXT_SECONDARY.blue,
            120,
        ))
        .font_size(10)
        .w(300.0);
}

/// Draw focus indicator around canvas region
pub fn draw_focus_indicator(draw: &Draw, rect: Rect) {
    draw.rect()
        .xy(rect.xy())
        .w_h(rect.w() + 4.0, rect.h() + 4.0)
        .no_fill()
        .stroke(colors::FOCUS_RING)
        .stroke_weight(2.0);
}

/// Draw toast notification
pub fn draw_toast(draw: &Draw, message: &str, alpha: f32, window_rect: Rect) {
    let toast_width = 300.0;
    let toast_height = 40.0;
    let pos = pt2(window_rect.x(), window_rect.bottom() + 60.0);

    let alpha_u8 = (alpha * 255.0) as u8;

    // Background
    draw.rect()
        .xy(pos)
        .w_h(toast_width, toast_height)
        .color(srgba(30, 28, 40, alpha_u8));

    // Border
    draw.rect()
        .xy(pos)
        .w_h(toast_width, toast_height)
        .no_fill()
        .stroke(srgba(
            colors::HUD_ACCENT.red,
            colors::HUD_ACCENT.green,
            colors::HUD_ACCENT.blue,
            alpha_u8,
        ))
        .stroke_weight(1.0);

    // Text
    draw.text(message)
        .xy(pos)
        .color(srgba(
            colors::TEXT_PRIMARY.red,
            colors::TEXT_PRIMARY.green,
            colors::TEXT_PRIMARY.blue,
            alpha_u8,
        ))
        .font_size(13)
        .w(toast_width - 20.0);
}

/// Draw error banner for TZ data issues
pub fn draw_error_banner(draw: &Draw, window_rect: Rect) {
    let banner_height = 40.0;
    let banner_y = window_rect.top() - banner_height / 2.0;

    // Background
    draw.rect()
        .x_y(window_rect.x(), banner_y)
        .w_h(window_rect.w(), banner_height)
        .color(srgba(120u8, 40u8, 40u8, 220u8));

    // Text
    draw.text("⚠ Timezone data may be missing or stale. Showing UTC as fallback.")
        .x_y(window_rect.x(), banner_y)
        .color(colors::TEXT_PRIMARY)
        .font_size(14)
        .w(window_rect.w() - 40.0);
}

