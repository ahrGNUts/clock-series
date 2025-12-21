//! Drawing module - ribbon rendering, DST seams, and visual effects
//!
//! Renders the worldline ribbon with its warm amber/sepia "paper scroll" aesthetic.

use nannou::prelude::*;
use shared::DstTransition;

use crate::ribbon::{RibbonViewport, Tick, TickType};

/// Color palette for the worldline ribbon theme - warm amber/sepia paper scroll aesthetic
pub mod colors {
    use nannou::prelude::*;

    /// Deep warm brown background
    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 26,
        green: 20,
        blue: 16,
        standard: std::marker::PhantomData,
    };

    /// Parchment ribbon surface (darker)
    pub const RIBBON_DARK: Srgb<u8> = Srgb {
        red: 30,
        green: 26,
        blue: 20,
        standard: std::marker::PhantomData,
    };

    /// Parchment ribbon surface (lighter)
    #[allow(dead_code)]
    pub const RIBBON_LIGHT: Srgb<u8> = Srgb {
        red: 42,
        green: 34,
        blue: 24,
        standard: std::marker::PhantomData,
    };

    /// Bright amber - now cursor
    pub const NOW_CURSOR: Srgb<u8> = Srgb {
        red: 255,
        green: 179,
        blue: 71,
        standard: std::marker::PhantomData,
    };

    /// Cream text primary
    pub const TEXT_PRIMARY: Srgb<u8> = Srgb {
        red: 245,
        green: 230,
        blue: 211,
        standard: std::marker::PhantomData,
    };

    /// Muted tan text secondary
    pub const TEXT_SECONDARY: Srgb<u8> = Srgb {
        red: 166,
        green: 144,
        blue: 128,
        standard: std::marker::PhantomData,
    };

    /// Warm gold for hour ticks
    pub const TICK_HOUR: Srgb<u8> = Srgb {
        red: 201,
        green: 168,
        blue: 108,
        standard: std::marker::PhantomData,
    };

    /// Medium gold for 5-minute ticks
    pub const TICK_FIVE_MIN: Srgb<u8> = Srgb {
        red: 140,
        green: 120,
        blue: 80,
        standard: std::marker::PhantomData,
    };

    /// Dim gold for minute ticks
    pub const TICK_MINUTE: Srgb<u8> = Srgb {
        red: 90,
        green: 78,
        blue: 55,
        standard: std::marker::PhantomData,
    };

    /// Very dim for second ticks
    pub const TICK_SECOND: Srgb<u8> = Srgb {
        red: 60,
        green: 52,
        blue: 40,
        standard: std::marker::PhantomData,
    };

    /// Sunset orange for DST seams
    pub const DST_SEAM: Srgb<u8> = Srgb {
        red: 255,
        green: 107,
        blue: 53,
        standard: std::marker::PhantomData,
    };

    /// Midnight marker (special)
    pub const MIDNIGHT: Srgb<u8> = Srgb {
        red: 180,
        green: 160,
        blue: 220,
        standard: std::marker::PhantomData,
    };

    /// Scrub mode indicator
    pub const SCRUB_MODE: Srgb<u8> = Srgb {
        red: 120,
        green: 180,
        blue: 220,
        standard: std::marker::PhantomData,
    };
}

/// Layout configuration for the ribbon
pub struct RibbonLayout {
    /// Center Y position of the ribbon
    pub ribbon_center_y: f32,
    /// Height of the ribbon band
    pub ribbon_height: f32,
    /// Height of tick marks (varies by type)
    pub tick_height_hour: f32,
    pub tick_height_five_min: f32,
    pub tick_height_minute: f32,
    pub tick_height_second: f32,
}

impl RibbonLayout {
    pub fn calculate(window_rect: Rect) -> Self {
        let ribbon_height = (window_rect.h() * 0.15).clamp(60.0, 120.0);
        Self {
            ribbon_center_y: 0.0,
            ribbon_height,
            tick_height_hour: ribbon_height * 0.6,
            tick_height_five_min: ribbon_height * 0.4,
            tick_height_minute: ribbon_height * 0.25,
            tick_height_second: ribbon_height * 0.15,
        }
    }
}

/// Draw the complete ribbon visualization
pub fn draw_ribbon(
    draw: &Draw,
    viewport: &RibbonViewport,
    ticks: &[Tick],
    transitions: &[DstTransition],
    layout: &RibbonLayout,
    is_scrub_mode: bool,
    reduced_motion: bool,
) {
    // Draw ribbon background
    draw_ribbon_background(draw, viewport, layout);

    // Draw DST seams first (so ticks appear on top)
    for transition in transitions {
        draw_dst_seam(draw, viewport, transition, layout, reduced_motion);
    }

    // Draw tick marks with warp effect
    for tick in ticks {
        let warped_x = if reduced_motion {
            tick.x_position
        } else {
            viewport.apply_warp(tick.x_position, tick.instant, transitions)
        };
        draw_tick(draw, tick, warped_x, layout);
    }

    // Draw the Now Cursor
    draw_now_cursor(draw, layout, is_scrub_mode);
}

fn draw_ribbon_background(draw: &Draw, viewport: &RibbonViewport, layout: &RibbonLayout) {
    let half_width = viewport.viewport_width / 2.0;

    // Main ribbon band with subtle gradient effect (using two overlapping rects)
    draw.rect()
        .x_y(0.0, layout.ribbon_center_y)
        .w_h(viewport.viewport_width + 20.0, layout.ribbon_height)
        .color(colors::RIBBON_DARK);

    // Lighter overlay in center for depth
    draw.rect()
        .x_y(0.0, layout.ribbon_center_y)
        .w_h(viewport.viewport_width * 0.6, layout.ribbon_height * 0.9)
        .color(srgba(42u8, 34u8, 24u8, 80u8));

    // Edge shadows
    let shadow_width = 80.0;
    for i in 0..20 {
        let alpha = ((20 - i) as f32 / 20.0 * 60.0) as u8;
        let offset = (i as f32 / 20.0) * shadow_width;

        // Left edge shadow
        draw.rect()
            .x_y(-half_width + offset / 2.0, layout.ribbon_center_y)
            .w_h(offset + 4.0, layout.ribbon_height)
            .color(srgba(26u8, 20u8, 16u8, alpha));

        // Right edge shadow
        draw.rect()
            .x_y(half_width - offset / 2.0, layout.ribbon_center_y)
            .w_h(offset + 4.0, layout.ribbon_height)
            .color(srgba(26u8, 20u8, 16u8, alpha));
    }

    // Top and bottom edge lines
    let edge_y = layout.ribbon_height / 2.0;
    draw.line()
        .start(pt2(-half_width, layout.ribbon_center_y + edge_y))
        .end(pt2(half_width, layout.ribbon_center_y + edge_y))
        .color(colors::TICK_MINUTE)
        .weight(1.0);

    draw.line()
        .start(pt2(-half_width, layout.ribbon_center_y - edge_y))
        .end(pt2(half_width, layout.ribbon_center_y - edge_y))
        .color(colors::TICK_MINUTE)
        .weight(1.0);
}

fn draw_tick(draw: &Draw, tick: &Tick, x: f32, layout: &RibbonLayout) {
    let (height, color, weight) = match tick.tick_type {
        TickType::Hour => (layout.tick_height_hour, colors::TICK_HOUR, 2.0),
        TickType::FiveMinute => (layout.tick_height_five_min, colors::TICK_FIVE_MIN, 1.5),
        TickType::Minute => (layout.tick_height_minute, colors::TICK_MINUTE, 1.0),
        TickType::Second => (layout.tick_height_second, colors::TICK_SECOND, 0.5),
        TickType::Midnight => (layout.tick_height_hour, colors::MIDNIGHT, 3.0),
    };

    let top = layout.ribbon_center_y + height / 2.0;
    let bottom = layout.ribbon_center_y - height / 2.0;

    draw.line()
        .start(pt2(x, top))
        .end(pt2(x, bottom))
        .color(color)
        .weight(weight);

    // Draw label if present
    if let Some(ref label) = tick.label {
        let label_y = match tick.tick_type {
            TickType::Midnight => layout.ribbon_center_y + layout.ribbon_height / 2.0 + 25.0,
            _ => layout.ribbon_center_y - layout.ribbon_height / 2.0 - 15.0,
        };

        let label_color = match tick.tick_type {
            TickType::Midnight => colors::MIDNIGHT,
            _ => colors::TEXT_SECONDARY,
        };

        draw.text(label)
            .x_y(x, label_y)
            .color(label_color)
            .font_size(12)
            .w(120.0);
    }
}

fn draw_dst_seam(
    draw: &Draw,
    viewport: &RibbonViewport,
    transition: &DstTransition,
    layout: &RibbonLayout,
    reduced_motion: bool,
) {
    let x = viewport.instant_to_x(transition.instant_utc);

    // Main seam line
    let seam_height = layout.ribbon_height * 1.5;
    draw.line()
        .start(pt2(x, layout.ribbon_center_y + seam_height / 2.0))
        .end(pt2(x, layout.ribbon_center_y - seam_height / 2.0))
        .color(colors::DST_SEAM)
        .weight(3.0);

    // Glow effect (unless reduced motion)
    if !reduced_motion {
        for i in 1..=5 {
            let alpha = (50 - i * 8) as u8;
            let offset = i as f32 * 2.0;
            draw.line()
                .start(pt2(x - offset, layout.ribbon_center_y + seam_height / 2.0))
                .end(pt2(x - offset, layout.ribbon_center_y - seam_height / 2.0))
                .color(srgba(255u8, 107u8, 53u8, alpha))
                .weight(1.0);
            draw.line()
                .start(pt2(x + offset, layout.ribbon_center_y + seam_height / 2.0))
                .end(pt2(x + offset, layout.ribbon_center_y - seam_height / 2.0))
                .color(srgba(255u8, 107u8, 53u8, alpha))
                .weight(1.0);
        }
    }

    // DST label
    let sign = if transition.delta_minutes > 0 { "+" } else { "" };
    let label = format!("DST {}{}m", sign, transition.delta_minutes);
    let label_y = layout.ribbon_center_y + seam_height / 2.0 + 20.0;

    draw.text(&label)
        .x_y(x, label_y)
        .color(colors::DST_SEAM)
        .font_size(14)
        .w(100.0);

    // Wall time labels (if not reduced motion)
    if !reduced_motion {
        let before_y = layout.ribbon_center_y - seam_height / 2.0 - 15.0;
        let after_y = layout.ribbon_center_y - seam_height / 2.0 - 30.0;

        draw.text(&format!("Before: {}", transition.local_wall_time_before))
            .x_y(x, before_y)
            .color(colors::TEXT_SECONDARY)
            .font_size(10)
            .w(180.0);

        draw.text(&format!("After: {}", transition.local_wall_time_after))
            .x_y(x, after_y)
            .color(colors::TEXT_SECONDARY)
            .font_size(10)
            .w(180.0);
    }
}

fn draw_now_cursor(draw: &Draw, layout: &RibbonLayout, is_scrub_mode: bool) {
    let cursor_height = layout.ribbon_height * 1.8;
    let top = layout.ribbon_center_y + cursor_height / 2.0;
    let bottom = layout.ribbon_center_y - cursor_height / 2.0;

    let cursor_color = if is_scrub_mode {
        colors::SCRUB_MODE
    } else {
        colors::NOW_CURSOR
    };

    // Main cursor line
    draw.line()
        .start(pt2(0.0, top))
        .end(pt2(0.0, bottom))
        .color(cursor_color)
        .weight(3.0);

    // Cursor head (triangle pointing down)
    let head_size = 12.0;
    let points = vec![
        pt2(0.0, top),
        pt2(-head_size, top + head_size),
        pt2(head_size, top + head_size),
    ];
    draw.polygon()
        .points(points)
        .color(cursor_color);

    // Cursor base (triangle pointing up)
    let base_points = vec![
        pt2(0.0, bottom),
        pt2(-head_size, bottom - head_size),
        pt2(head_size, bottom - head_size),
    ];
    draw.polygon()
        .points(base_points)
        .color(cursor_color);
}

/// Draw the time display above the ribbon
pub fn draw_time_display(
    draw: &Draw,
    time_text: &str,
    date_text: &str,
    layout: &RibbonLayout,
    is_scrub_mode: bool,
) {
    // Position time display well above the ribbon and cursor triangle
    let time_y = layout.ribbon_center_y + layout.ribbon_height + 120.0;
    let date_y = time_y + 50.0;

    let time_color = if is_scrub_mode {
        colors::SCRUB_MODE
    } else {
        colors::TEXT_PRIMARY
    };

    draw.text(time_text)
        .x_y(0.0, time_y)
        .color(time_color)
        .font_size(48)
        .w(400.0);

    draw.text(date_text)
        .x_y(0.0, date_y)
        .color(colors::TEXT_SECONDARY)
        .font_size(20)
        .w(400.0);

    // Scrub mode indicator - positioned above the time
    if is_scrub_mode {
        let indicator_y = time_y - 40.0;
        draw.text("◆ SCRUB MODE ◆")
            .x_y(0.0, indicator_y)
            .color(colors::SCRUB_MODE)
            .font_size(12)
            .w(200.0);
    }
}

/// Draw zoom level indicator
pub fn draw_zoom_indicator(draw: &Draw, seconds_per_pixel: f32, window_rect: Rect) {
    let text = format!("{:.0} sec/px", seconds_per_pixel);
    let x = window_rect.left() + 80.0;
    let y = window_rect.bottom() + 30.0;

    draw.text(&text)
        .x_y(x, y)
        .color(colors::TEXT_SECONDARY)
        .font_size(12)
        .w(100.0);
}

/// Draw keyboard shortcuts help
pub fn draw_help_text(draw: &Draw, window_rect: Rect) {
    let help_lines = [
        "Space: Toggle Live/Scrub",
        "←/→: ±1 sec  |  Shift: ±1 min  |  Ctrl: ±1 hr",
        "Ctrl+/Ctrl-: Zoom  |  /: Search TZ",
    ];

    let x = 0.0;
    let base_y = window_rect.bottom() + 60.0;

    for (i, line) in help_lines.iter().enumerate() {
        draw.text(line)
            .x_y(x, base_y + (help_lines.len() - 1 - i) as f32 * 16.0)
            .color(srgba(166u8, 144u8, 128u8, 120u8))
            .font_size(11)
            .w(500.0);
    }
}

