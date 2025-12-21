//! Drawing module - calibration ring and time display rendering
//!
//! Renders the precision instrument clock visual elements using nannou's Draw API.

use nannou::prelude::*;
use shared::{DstChange, TimeData};

/// Color palette for the precision instrument theme
pub mod colors {
    use nannou::prelude::*;

    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 26,
        green: 26,
        blue: 26,
        standard: std::marker::PhantomData,
    };
    pub const TEXT_PRIMARY: Srgb<u8> = Srgb {
        red: 240,
        green: 240,
        blue: 240,
        standard: std::marker::PhantomData,
    };
    pub const TEXT_SECONDARY: Srgb<u8> = Srgb {
        red: 160,
        green: 160,
        blue: 160,
        standard: std::marker::PhantomData,
    };
    pub const ACCENT: Srgb<u8> = Srgb {
        red: 0,
        green: 212,
        blue: 255,
        standard: std::marker::PhantomData,
    };
    pub const ACCENT_DIM: Srgb<u8> = Srgb {
        red: 0,
        green: 106,
        blue: 128,
        standard: std::marker::PhantomData,
    };
    #[allow(dead_code)]
    pub const WARNING: Srgb<u8> = Srgb {
        red: 255,
        green: 180,
        blue: 0,
        standard: std::marker::PhantomData,
    };
    pub const TICK_NORMAL: Srgb<u8> = Srgb {
        red: 80,
        green: 80,
        blue: 80,
        standard: std::marker::PhantomData,
    };
    pub const TICK_MAJOR: Srgb<u8> = Srgb {
        red: 120,
        green: 120,
        blue: 120,
        standard: std::marker::PhantomData,
    };
}

/// Draw the primary time readout (left panel)
pub fn draw_primary_readout(draw: &Draw, time_data: &TimeData, rect: Rect) {
    let center = rect.xy();
    
    // Large time display: hh:mm:ss with AM/PM as superscript
    let time_str = time_data.format_time();
    let meridiem_str = time_data.meridiem.to_string();
    
    // Offset time slightly left to make room for AM/PM
    let time_x_offset = -20.0;
    let time_y = 60.0;
    
    draw.text(&time_str)
        .xy(center + vec2(time_x_offset, time_y))
        .color(colors::TEXT_PRIMARY)
        .font_size(72)
        .w(rect.w());
    
    // AM/PM indicator - positioned as superscript to the right of time
    // Approximate time text width: 8 chars * ~40px = ~160px half-width
    let time_half_width = 160.0;
    let am_pm_x = time_x_offset + time_half_width + 8.0;
    let am_pm_y = time_y + 18.0; // Align with upper portion of digits
    
    draw.text(&meridiem_str)
        .xy(center + vec2(am_pm_x, am_pm_y))
        .color(colors::ACCENT)
        .font_size(24)
        .w(100.0);
    
    // Date line
    let date_str = time_data.format_date();
    draw.text(&date_str)
        .xy(center + vec2(0.0, 0.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(20)
        .w(rect.w());
    
    // Timezone info line
    let tz_str = format!(
        "{} · {} · DST {}",
        time_data.tz_abbrev,
        time_data.format_utc_offset(),
        if time_data.is_dst { "On" } else { "Off" }
    );
    draw.text(&tz_str)
        .xy(center + vec2(0.0, -40.0))
        .color(colors::TEXT_SECONDARY)
        .font_size(16)
        .w(rect.w());
}

/// Draw the calibration ring (right panel)
pub fn draw_calibration_ring(
    draw: &Draw,
    time_data: &TimeData,
    center: Point2,
    radius: f32,
    reduced_motion: bool,
) {
    let num_ticks = 60;
    let tick_length_minor = radius * 0.08;
    let tick_length_major = radius * 0.15;
    
    // Draw tick marks
    for i in 0..num_ticks {
        // Start at 12 o'clock (PI/2) and go clockwise (subtract angle)
        let angle = PI / 2.0 - (i as f32 / num_ticks as f32) * TAU;
        let is_major = i % 5 == 0;
        let is_current = i == time_data.second as usize;
        
        let tick_length = if is_major { tick_length_major } else { tick_length_minor };
        let inner_radius = radius - tick_length;
        
        let start = center + vec2(angle.cos(), angle.sin()) * inner_radius;
        let end = center + vec2(angle.cos(), angle.sin()) * radius;
        
        let color = if is_current {
            colors::ACCENT
        } else if is_major {
            colors::TICK_MAJOR
        } else {
            colors::TICK_NORMAL
        };
        
        let weight = if is_current {
            3.0
        } else if is_major {
            2.0
        } else {
            1.0
        };
        
        draw.line()
            .start(start)
            .end(end)
            .color(color)
            .weight(weight);
    }
    
    // Draw sweeping second indicator (smooth animation)
    if !reduced_motion {
        let second_with_fraction = time_data.second as f64 + time_data.second_fraction;
        // Start at 12 o'clock (PI/2) and go clockwise (subtract angle)
        let sweep_angle = PI / 2.0 - (second_with_fraction / 60.0) as f32 * TAU;
        
        let indicator_pos = center + vec2(sweep_angle.cos(), sweep_angle.sin()) * (radius * 0.7);
        
        // Glow effect (larger, dimmer circle behind)
        draw.ellipse()
            .xy(indicator_pos)
            .radius(8.0)
            .color(srgba(0u8, 212u8, 255u8, 80u8));
        
        // Main indicator
        draw.ellipse()
            .xy(indicator_pos)
            .radius(4.0)
            .color(colors::ACCENT);
    }
    
    // Draw center dot
    draw.ellipse()
        .xy(center)
        .radius(3.0)
        .color(colors::ACCENT_DIM);
    
    // Draw outer ring
    draw_ring(draw, center, radius, 1.5, colors::TICK_MAJOR);
    draw_ring(draw, center, radius - tick_length_major - 5.0, 0.5, colors::TICK_NORMAL);
}

/// Draw a ring (circle outline) using line segments
fn draw_ring(draw: &Draw, center: Point2, radius: f32, weight: f32, color: Srgb<u8>) {
    let segments = 120;
    let points: Vec<Point2> = (0..=segments)
        .map(|i| {
            let angle = (i as f32 / segments as f32) * TAU;
            center + vec2(angle.cos(), angle.sin()) * radius
        })
        .collect();
    
    draw.polyline()
        .weight(weight)
        .color(color)
        .points(points);
}

/// Draw DST status indicator
#[allow(dead_code)]
pub fn draw_dst_status(draw: &Draw, time_data: &TimeData, position: Point2) {
    let status_text = match &time_data.dst_change {
        DstChange::None => {
            if time_data.is_dst {
                "DST Active".to_string()
            } else {
                "Standard Time".to_string()
            }
        }
        DstChange::Upcoming { delta_minutes, .. } => {
            let direction = if *delta_minutes > 0 {
                "forward"
            } else {
                "back"
            };
            format!(
                "DST change: {} {}min in <24h",
                direction,
                delta_minutes.abs()
            )
        }
        DstChange::JustOccurred { delta_minutes, .. } => {
            let direction = if *delta_minutes > 0 {
                "forward"
            } else {
                "back"
            };
            format!(
                "DST changed: {} {}min",
                direction,
                delta_minutes.abs()
            )
        }
    };
    
    let color = match &time_data.dst_change {
        DstChange::None => colors::TEXT_SECONDARY,
        DstChange::Upcoming { .. } => colors::WARNING,
        DstChange::JustOccurred { .. } => colors::ACCENT,
    };
    
    draw.text(&status_text)
        .xy(position)
        .color(color)
        .font_size(14)
        .w(300.0);
}

/// Draw the error banner when timezone data is invalid
pub fn draw_error_banner(draw: &Draw, message: &str, rect: Rect) {
    let banner_height = 40.0;
    let banner_rect = Rect::from_x_y_w_h(
        rect.x(),
        rect.top() - banner_height / 2.0,
        rect.w(),
        banner_height,
    );
    
    // Background
    draw.rect()
        .xy(banner_rect.xy())
        .wh(banner_rect.wh())
        .color(srgb(80u8, 20u8, 20u8));
    
    // Text
    draw.text(message)
        .xy(banner_rect.xy())
        .color(colors::TEXT_PRIMARY)
        .font_size(14)
        .w(banner_rect.w() - 20.0);
}

/// Calculate layout rectangles for two-column layout
pub struct Layout {
    pub left_panel: Rect,
    pub right_panel: Rect,
    #[allow(dead_code)]
    pub is_single_column: bool,
}

impl Layout {
    pub fn calculate(window_rect: Rect) -> Self {
        let padding = 40.0;
        let inner = window_rect.pad(padding);
        
        // Switch to single column below 640px width
        let is_single_column = window_rect.w() < 640.0;
        
        if is_single_column {
            // Stack vertically
            let half_height = inner.h() / 2.0;
            Layout {
                left_panel: Rect::from_x_y_w_h(
                    inner.x(),
                    inner.y() + half_height / 2.0,
                    inner.w(),
                    half_height,
                ),
                right_panel: Rect::from_x_y_w_h(
                    inner.x(),
                    inner.y() - half_height / 2.0,
                    inner.w(),
                    half_height,
                ),
                is_single_column: true,
            }
        } else {
            // Side by side
            let half_width = inner.w() / 2.0;
            Layout {
                left_panel: Rect::from_x_y_w_h(
                    inner.left() + half_width / 2.0,
                    inner.y(),
                    half_width,
                    inner.h(),
                ),
                right_panel: Rect::from_x_y_w_h(
                    inner.right() - half_width / 2.0,
                    inner.y(),
                    half_width,
                    inner.h(),
                ),
                is_single_column: false,
            }
        }
    }
}
