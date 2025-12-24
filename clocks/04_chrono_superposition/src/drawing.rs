//! Drawing module - Card deck rendering, composite readout, and visual effects
//!
//! Handles all nannou-based rendering including the superposition deck,
//! composite readout, list view, and DST warning effects.

use std::collections::HashMap;

use chrono_tz::Tz;
use nannou::prelude::*;
use shared::{DstChange, TimeData};

use crate::cards::{CardGeometry, ZoneComparison, CARD_HEIGHT, CARD_WIDTH};

/// Color palette for the chrono-superposition theme
#[allow(dead_code)]
pub mod colors {
    use nannou::prelude::*;

    /// Deep background
    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 18,
        green: 22,
        blue: 28,
        standard: std::marker::PhantomData,
    };

    /// Card background (dominant)
    pub const CARD_BG_DOMINANT: Srgb<u8> = Srgb {
        red: 45,
        green: 52,
        blue: 65,
        standard: std::marker::PhantomData,
    };

    /// Card background (others)
    pub const CARD_BG: Srgb<u8> = Srgb {
        red: 35,
        green: 40,
        blue: 50,
        standard: std::marker::PhantomData,
    };

    /// Card border
    pub const CARD_BORDER: Srgb<u8> = Srgb {
        red: 80,
        green: 90,
        blue: 110,
        standard: std::marker::PhantomData,
    };

    /// Card border (dominant)
    pub const CARD_BORDER_DOMINANT: Srgb<u8> = Srgb {
        red: 120,
        green: 140,
        blue: 180,
        standard: std::marker::PhantomData,
    };

    /// DST warning border
    pub const DST_WARNING: Srgb<u8> = Srgb {
        red: 255,
        green: 107,
        blue: 53,
        standard: std::marker::PhantomData,
    };

    /// DST active indicator
    pub const DST_ACTIVE: Srgb<u8> = Srgb {
        red: 255,
        green: 179,
        blue: 71,
        standard: std::marker::PhantomData,
    };

    /// Time text color
    pub const TIME_TEXT: Srgb<u8> = Srgb {
        red: 245,
        green: 240,
        blue: 235,
        standard: std::marker::PhantomData,
    };

    /// Date/secondary text
    pub const SECONDARY_TEXT: Srgb<u8> = Srgb {
        red: 160,
        green: 165,
        blue: 175,
        standard: std::marker::PhantomData,
    };

    /// Zone name text
    pub const ZONE_TEXT: Srgb<u8> = Srgb {
        red: 140,
        green: 150,
        blue: 165,
        standard: std::marker::PhantomData,
    };

    /// Primary text
    pub const TEXT_PRIMARY: Srgb<u8> = Srgb {
        red: 220,
        green: 225,
        blue: 235,
        standard: std::marker::PhantomData,
    };

    /// Positive delta
    pub const DELTA_POSITIVE: Srgb<u8> = Srgb {
        red: 100,
        green: 200,
        blue: 150,
        standard: std::marker::PhantomData,
    };

    /// Negative delta
    pub const DELTA_NEGATIVE: Srgb<u8> = Srgb {
        red: 200,
        green: 120,
        blue: 120,
        standard: std::marker::PhantomData,
    };

    /// Composite background
    pub const COMPOSITE_BG: Srgb<u8> = Srgb {
        red: 30,
        green: 35,
        blue: 45,
        standard: std::marker::PhantomData,
    };

    /// List item background
    pub const LIST_ITEM_BG: Srgb<u8> = Srgb {
        red: 32,
        green: 38,
        blue: 48,
        standard: std::marker::PhantomData,
    };

    /// List item dominant background
    pub const LIST_ITEM_DOMINANT: Srgb<u8> = Srgb {
        red: 42,
        green: 50,
        blue: 65,
        standard: std::marker::PhantomData,
    };
}

/// Layout configuration for the core (center) area
#[derive(Debug, Clone)]
pub struct CoreLayout {
    /// Left edge of the core area
    pub left: f32,
    /// Right edge of the core area
    pub right: f32,
    /// Top edge
    pub top: f32,
    /// Bottom edge
    pub bottom: f32,
    /// Width of the core area
    pub width: f32,
    /// Height of the core area
    #[allow(dead_code)]
    pub height: f32,
    /// Center X position
    pub center_x: f32,
    /// Center Y position
    pub center_y: f32,
}

impl CoreLayout {
    /// Calculate layout from window dimensions, accounting for left and right panels
    pub fn calculate(window_rect: Rect, left_panel_width: f32, right_panel_width: f32) -> Self {
        let margin = 20.0;
        let left = window_rect.left() + left_panel_width + margin;
        let right = window_rect.right() - right_panel_width - margin;
        let top = window_rect.top() - 60.0; // Leave room for title
        let bottom = window_rect.bottom() + margin;

        let width = right - left;
        let height = top - bottom;
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        Self {
            left,
            right,
            top,
            bottom,
            width,
            height,
            center_x,
            center_y,
        }
    }

    /// Check if a point is within the core area
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.bottom && y <= self.top
    }
}

/// Draw the card deck view
pub fn draw_card_deck(
    draw: &Draw,
    layout: &CoreLayout,
    display_order: &[Tz],
    zone_times: &HashMap<Tz, TimeData>,
    dominant_zone: Tz,
    geometries: &[CardGeometry],
    compare_mode: bool,
    hovered_index: Option<usize>,
    animation_time: f32,
    reduced_motion: bool,
) {
    // Get dominant zone data for comparison
    let dominant_data = zone_times.get(&dominant_zone);

    // Draw cards from back to front (reverse order so dominant is on top)
    for i in (0..display_order.len()).rev() {
        let tz = display_order[i];
        let geom = &geometries[i];
        let is_dominant = tz == dominant_zone;
        let is_hovered = hovered_index == Some(i);

        if let Some(time_data) = zone_times.get(&tz) {
            draw_zone_card(
                draw,
                layout,
                tz,
                time_data,
                geom,
                is_dominant,
                is_hovered,
                compare_mode,
                dominant_data,
                animation_time,
                reduced_motion,
            );
        }
    }
}

/// Draw a single zone card
fn draw_zone_card(
    draw: &Draw,
    layout: &CoreLayout,
    tz: Tz,
    time_data: &TimeData,
    geom: &CardGeometry,
    is_dominant: bool,
    is_hovered: bool,
    compare_mode: bool,
    dominant_data: Option<&TimeData>,
    animation_time: f32,
    reduced_motion: bool,
) {
    let card_x = layout.center_x + geom.offset.x;
    let card_y = layout.center_y + geom.offset.y;
    let card_w = CARD_WIDTH * geom.scale;
    let card_h = CARD_HEIGHT * geom.scale;

    // Determine if DST warning should pulse
    let has_dst_warning = matches!(
        time_data.dst_change,
        DstChange::Upcoming { .. } | DstChange::JustOccurred { .. }
    );
    let pulse = if has_dst_warning && !reduced_motion {
        (animation_time * 3.0).sin() * 0.5 + 0.5
    } else {
        0.0
    };

    // Draw card shadow
    draw.rect()
        .x_y(card_x + 4.0, card_y - 4.0)
        .w_h(card_w, card_h)
        .rotate(geom.rotation)
        .color(srgba(0, 0, 0, (60.0 * geom.opacity) as u8));

    // Card background
    let bg_color = if is_dominant {
        colors::CARD_BG_DOMINANT
    } else {
        colors::CARD_BG
    };
    draw.rect()
        .x_y(card_x, card_y)
        .w_h(card_w, card_h)
        .rotate(geom.rotation)
        .color(srgba(
            bg_color.red,
            bg_color.green,
            bg_color.blue,
            (255.0 * geom.opacity) as u8,
        ));

    // Card border
    let border_color = if has_dst_warning {
        let r = colors::CARD_BORDER.red as f32
            + (colors::DST_WARNING.red as f32 - colors::CARD_BORDER.red as f32) * pulse;
        let g = colors::CARD_BORDER.green as f32
            + (colors::DST_WARNING.green as f32 - colors::CARD_BORDER.green as f32) * pulse;
        let b = colors::CARD_BORDER.blue as f32
            + (colors::DST_WARNING.blue as f32 - colors::CARD_BORDER.blue as f32) * pulse;
        srgba(r as u8, g as u8, b as u8, (255.0 * geom.opacity) as u8)
    } else if is_dominant {
        srgba(
            colors::CARD_BORDER_DOMINANT.red,
            colors::CARD_BORDER_DOMINANT.green,
            colors::CARD_BORDER_DOMINANT.blue,
            (255.0 * geom.opacity) as u8,
        )
    } else if is_hovered {
        srgba(
            colors::CARD_BORDER_DOMINANT.red,
            colors::CARD_BORDER_DOMINANT.green,
            colors::CARD_BORDER_DOMINANT.blue,
            (200.0 * geom.opacity) as u8,
        )
    } else {
        srgba(
            colors::CARD_BORDER.red,
            colors::CARD_BORDER.green,
            colors::CARD_BORDER.blue,
            (180.0 * geom.opacity) as u8,
        )
    };

    // Draw border as 4 lines (rotated rect outline)
    let half_w = card_w / 2.0;
    let half_h = card_h / 2.0;
    let corners = [
        pt2(-half_w, -half_h),
        pt2(half_w, -half_h),
        pt2(half_w, half_h),
        pt2(-half_w, half_h),
    ];

    // Rotate corners
    let cos_r = geom.rotation.cos();
    let sin_r = geom.rotation.sin();
    let rotated: Vec<Point2> = corners
        .iter()
        .map(|p| {
            pt2(
                card_x + p.x * cos_r - p.y * sin_r,
                card_y + p.x * sin_r + p.y * cos_r,
            )
        })
        .collect();

    for i in 0..4 {
        let next = (i + 1) % 4;
        draw.line()
            .start(rotated[i])
            .end(rotated[next])
            .color(border_color)
            .stroke_weight(if is_dominant { 2.0 } else { 1.0 });
    }

    // Content (no rotation for readability)
    let content_scale = geom.scale;
    let text_opacity = (255.0 * geom.opacity) as u8;

    // Zone name
    draw.text(tz.name())
        .x_y(card_x, card_y + card_h * 0.32)
        .w(card_w - 20.0)
        .color(srgba(
            colors::ZONE_TEXT.red,
            colors::ZONE_TEXT.green,
            colors::ZONE_TEXT.blue,
            text_opacity,
        ))
        .font_size((11.0 * content_scale) as u32)
        .center_justify();

    // Time
    let time_str = format!(
        "{}:{:02}:{:02}",
        time_data.hour12, time_data.minute, time_data.second
    );
    draw.text(&time_str)
        .x_y(card_x, card_y + card_h * 0.08)
        .color(srgba(
            colors::TIME_TEXT.red,
            colors::TIME_TEXT.green,
            colors::TIME_TEXT.blue,
            text_opacity,
        ))
        .font_size((28.0 * content_scale) as u32);

    // Meridiem
    draw.text(&time_data.meridiem.to_string())
        .x_y(card_x + card_w * 0.32, card_y + card_h * 0.08)
        .color(srgba(
            colors::SECONDARY_TEXT.red,
            colors::SECONDARY_TEXT.green,
            colors::SECONDARY_TEXT.blue,
            text_opacity,
        ))
        .font_size((14.0 * content_scale) as u32);

    // Date
    let date_str = format!(
        "{} {}, {}",
        month_abbrev(time_data.month),
        time_data.day,
        time_data.year
    );
    draw.text(&date_str)
        .x_y(card_x, card_y - card_h * 0.12)
        .color(srgba(
            colors::SECONDARY_TEXT.red,
            colors::SECONDARY_TEXT.green,
            colors::SECONDARY_TEXT.blue,
            text_opacity,
        ))
        .font_size((12.0 * content_scale) as u32);

    // Offset + DST badge
    let offset_str = time_data.format_utc_offset();
    let dst_str = if time_data.is_dst { " DST" } else { "" };
    draw.text(&format!("{}{}", offset_str, dst_str))
        .x_y(card_x, card_y - card_h * 0.28)
        .color(srgba(
            if time_data.is_dst {
                colors::DST_ACTIVE.red
            } else {
                colors::SECONDARY_TEXT.red
            },
            if time_data.is_dst {
                colors::DST_ACTIVE.green
            } else {
                colors::SECONDARY_TEXT.green
            },
            if time_data.is_dst {
                colors::DST_ACTIVE.blue
            } else {
                colors::SECONDARY_TEXT.blue
            },
            text_opacity,
        ))
        .font_size((11.0 * content_scale) as u32);

    // Compare mode deltas (if not dominant)
    if compare_mode && !is_dominant {
        if let Some(dom_data) = dominant_data {
            let comparison = ZoneComparison::compute(
                time_data.utc_offset_minutes,
                time_data.is_dst,
                compute_day_index(time_data, dom_data),
                dom_data.utc_offset_minutes,
                dom_data.is_dst,
                0, // dominant is always day 0
            );

            draw_comparison_badge(
                draw,
                card_x + card_w * 0.35,
                card_y - card_h * 0.35,
                &comparison,
                content_scale,
                text_opacity,
            );
        }
    }

    // DST warning indicator
    if has_dst_warning {
        draw_dst_warning_badge(
            draw,
            card_x - card_w * 0.35,
            card_y + card_h * 0.35,
            time_data,
            content_scale,
            text_opacity,
        );
    }
}

/// Draw comparison delta badge
fn draw_comparison_badge(
    draw: &Draw,
    x: f32,
    y: f32,
    comparison: &ZoneComparison,
    scale: f32,
    opacity: u8,
) {
    let mut badges: Vec<(String, Srgb<u8>)> = Vec::new();

    // Hours delta
    let hours_str = comparison.format_hours();
    if !hours_str.is_empty() {
        let color = if comparison.delta_hours > 0 {
            colors::DELTA_POSITIVE
        } else {
            colors::DELTA_NEGATIVE
        };
        badges.push((hours_str, color));
    }

    // Day delta
    if let Some(day_str) = comparison.format_day() {
        badges.push((day_str.to_string(), colors::SECONDARY_TEXT));
    }

    // DST differs
    if comparison.dst_differs {
        badges.push(("DST differs".to_string(), colors::DST_WARNING));
    }

    // Draw badges
    let mut offset_y = 0.0;
    for (text, color) in badges {
        draw.text(&text)
            .x_y(x, y - offset_y)
            .color(srgba(color.red, color.green, color.blue, opacity))
            .font_size((10.0 * scale) as u32);
        offset_y += 12.0 * scale;
    }
}

/// Draw DST warning badge
fn draw_dst_warning_badge(
    draw: &Draw,
    x: f32,
    y: f32,
    time_data: &TimeData,
    scale: f32,
    opacity: u8,
) {
    let now = chrono::Utc::now();
    let warning_text = match &time_data.dst_change {
        DstChange::Upcoming { instant, delta_minutes } => {
            let hours_remaining = (*instant - now).num_hours();
            let direction = if *delta_minutes > 0 { "+" } else { "" };
            format!("DST in {}h ({}{}m)", hours_remaining, direction, delta_minutes)
        }
        DstChange::JustOccurred { instant, delta_minutes } => {
            let hours_ago = (now - *instant).num_hours();
            let direction = if *delta_minutes > 0 { "+" } else { "" };
            format!("DST {}h ago ({}{}m)", hours_ago, direction, delta_minutes)
        }
        DstChange::None => return,
    };

    draw.text(&warning_text)
        .x_y(x, y)
        .color(srgba(
            colors::DST_WARNING.red,
            colors::DST_WARNING.green,
            colors::DST_WARNING.blue,
            opacity,
        ))
        .font_size((9.0 * scale) as u32);
}

/// Draw the composite readout view (when focus_strength >= 0.8)
pub fn draw_composite_readout(
    draw: &Draw,
    layout: &CoreLayout,
    display_order: &[Tz],
    zone_times: &HashMap<Tz, TimeData>,
    dominant_zone: Tz,
    compare_mode: bool,
    animation_time: f32,
) {
    // Compute composite data
    let composite = compute_composite_data(display_order, zone_times, dominant_zone);

    // Background panel
    let panel_w = 400.0;
    let panel_h = 280.0;
    draw.rect()
        .x_y(layout.center_x, layout.center_y)
        .w_h(panel_w, panel_h)
        .color(colors::COMPOSITE_BG);

    draw.rect()
        .x_y(layout.center_x, layout.center_y)
        .w_h(panel_w, panel_h)
        .no_fill()
        .stroke(colors::CARD_BORDER_DOMINANT)
        .stroke_weight(2.0);

    // Title
    draw.text("SUPERPOSITION COLLAPSED")
        .x_y(layout.center_x, layout.center_y + panel_h * 0.38)
        .color(colors::ZONE_TEXT)
        .font_size(11);

    // Time (with range if different hours)
    draw.text(&composite.time_display)
        .x_y(layout.center_x, layout.center_y + panel_h * 0.15)
        .color(colors::TIME_TEXT)
        .font_size(42);

    // Meridiem
    draw.text(&composite.meridiem_display)
        .x_y(layout.center_x + 140.0, layout.center_y + panel_h * 0.15)
        .color(colors::SECONDARY_TEXT)
        .font_size(18);

    // Date display
    draw.text(&composite.date_display)
        .x_y(layout.center_x, layout.center_y - panel_h * 0.05)
        .color(colors::SECONDARY_TEXT)
        .font_size(14);

    // Date badges for zones with different dates
    if !composite.date_badges.is_empty() {
        let badge_y = layout.center_y - panel_h * 0.12;
        for (i, (zone_name, badge)) in composite.date_badges.iter().take(3).enumerate() {
            let badge_text = format!("{}: {}", zone_name, badge);
            draw.text(&badge_text)
                .x_y(layout.center_x, badge_y - (i as f32 * 14.0))
                .color(colors::DST_ACTIVE)
                .font_size(10);
        }
        if composite.date_badges.len() > 3 {
            draw.text(&format!("...and {} more", composite.date_badges.len() - 3))
                .x_y(layout.center_x, badge_y - 42.0)
                .color(colors::ZONE_TEXT)
                .font_size(9);
        }
    }

    // Zone count
    let zone_count_str = format!("{} zones superposed", display_order.len());
    draw.text(&zone_count_str)
        .x_y(layout.center_x, layout.center_y - panel_h * 0.28)
        .color(colors::ZONE_TEXT)
        .font_size(12);

    // DST warning if any zone has transition
    if composite.has_dst_warning {
        let pulse = (animation_time * 3.0).sin() * 0.5 + 0.5;
        let alpha = (180.0 + 75.0 * pulse) as u8;
        draw.text("⚠ DST transition imminent in some zones")
            .x_y(layout.center_x, layout.center_y - panel_h * 0.35)
            .color(srgba(
                colors::DST_WARNING.red,
                colors::DST_WARNING.green,
                colors::DST_WARNING.blue,
                alpha,
            ))
            .font_size(11);
    }

    // Compare mode: show all zones as small list
    if compare_mode {
        draw_composite_zone_list(
            draw,
            layout.center_x,
            layout.center_y - panel_h * 0.55,
            display_order,
            zone_times,
            dominant_zone,
        );
    }
}

/// Composite display data
struct CompositeData {
    time_display: String,
    meridiem_display: String,
    date_display: String,
    /// Date badges for zones with different dates (e.g., "Yesterday", "Tomorrow")
    date_badges: Vec<(String, &'static str)>, // (zone_short_name, badge)
    has_dst_warning: bool,
}

/// Compute composite readout data
fn compute_composite_data(
    display_order: &[Tz],
    zone_times: &HashMap<Tz, TimeData>,
    dominant_zone: Tz,
) -> CompositeData {
    let dominant_data = zone_times.get(&dominant_zone);

    // Collect all zone data with their tz
    let all_data: Vec<(Tz, &TimeData)> = display_order
        .iter()
        .filter_map(|&tz| zone_times.get(&tz).map(|td| (tz, td)))
        .collect();

    if all_data.is_empty() {
        return CompositeData {
            time_display: "--:--:--".to_string(),
            meridiem_display: "".to_string(),
            date_display: "No data".to_string(),
            date_badges: Vec::new(),
            has_dst_warning: false,
        };
    }

    // Use dominant zone's minute and second (they're aligned)
    let minute = all_data[0].1.minute;
    let second = all_data[0].1.second;

    // Collect unique hours and meridiems
    let mut hours: Vec<u32> = all_data.iter().map(|(_, d)| d.hour12).collect();
    hours.sort();
    hours.dedup();

    let mut meridiems: Vec<_> = all_data.iter().map(|(_, d)| d.meridiem).collect();
    meridiems.dedup();

    // Time display
    let time_display = if hours.len() == 1 {
        format!("{}:{:02}:{:02}", hours[0], minute, second)
    } else {
        let min_h = hours.first().unwrap();
        let max_h = hours.last().unwrap();
        format!("{}-{}:{:02}:{:02}", min_h, max_h, minute, second)
    };

    // Meridiem display
    let meridiem_display = if meridiems.len() == 1 {
        meridiems[0].to_string()
    } else {
        "AM–PM".to_string()
    };

    // Date display and badges - check if dates differ
    let dominant_date = dominant_data.map(|d| (d.year, d.month, d.day));
    let dates_same = all_data
        .iter()
        .all(|(_, d)| Some((d.year, d.month, d.day)) == dominant_date);

    let mut date_badges: Vec<(String, &'static str)> = Vec::new();

    let date_display = if dates_same {
        if let Some(d) = dominant_data {
            format!("{} {}, {}", month_abbrev(d.month), d.day, d.year)
        } else {
            "Today".to_string()
        }
    } else {
        // Compute date badges for zones with different dates
        if let Some(dom_data) = dominant_data {
            for (tz, td) in &all_data {
                if *tz == dominant_zone {
                    continue;
                }
                let day_diff = compute_day_index(td, dom_data);
                if day_diff != 0 {
                    let short_name: String = tz
                        .name()
                        .split('/')
                        .last()
                        .unwrap_or(tz.name())
                        .chars()
                        .take(10)
                        .collect();
                    let badge = match day_diff {
                        -1 => "Yesterday",
                        1 => "Tomorrow",
                        _ => "Different day",
                    };
                    date_badges.push((short_name, badge));
                }
            }
        }

        if date_badges.is_empty() {
            if let Some(d) = dominant_data {
                format!("{} {}, {}", month_abbrev(d.month), d.day, d.year)
            } else {
                "Today".to_string()
            }
        } else {
            // Show dominant date with indicator
            if let Some(d) = dominant_data {
                format!("{} {}, {} (varies)", month_abbrev(d.month), d.day, d.year)
            } else {
                "Multiple dates".to_string()
            }
        }
    };

    // Check for DST warnings
    let has_dst_warning = all_data.iter().any(|(_, d)| {
        matches!(
            d.dst_change,
            DstChange::Upcoming { .. } | DstChange::JustOccurred { .. }
        )
    });

    CompositeData {
        time_display,
        meridiem_display,
        date_display,
        date_badges,
        has_dst_warning,
    }
}

/// Draw a compact list of zones for compare mode in composite view
fn draw_composite_zone_list(
    draw: &Draw,
    x: f32,
    y: f32,
    display_order: &[Tz],
    zone_times: &HashMap<Tz, TimeData>,
    dominant_zone: Tz,
) {
    let dominant_data = zone_times.get(&dominant_zone);
    let item_height = 16.0;
    let max_display = 6;

    for (i, &tz) in display_order.iter().take(max_display).enumerate() {
        let item_y = y - (i as f32) * item_height;
        let is_dominant = tz == dominant_zone;

        if let Some(time_data) = zone_times.get(&tz) {
            // Zone name
            let name_color = if is_dominant {
                colors::TIME_TEXT
            } else {
                colors::ZONE_TEXT
            };

            // Format short zone name
            let short_name: String = tz
                .name()
                .split('/')
                .last()
                .unwrap_or(tz.name())
                .chars()
                .take(15)
                .collect();

            draw.text(&short_name)
                .x_y(x - 100.0, item_y)
                .color(name_color)
                .font_size(10)
                .left_justify();

            // Time
            let time_str = format!("{}:{:02} {}", time_data.hour12, time_data.minute, time_data.meridiem);
            draw.text(&time_str)
                .x_y(x + 50.0, item_y)
                .color(colors::SECONDARY_TEXT)
                .font_size(10);

            // Delta (if not dominant)
            if !is_dominant {
                if let Some(dom_data) = dominant_data {
                    let delta_hours =
                        (time_data.utc_offset_minutes - dom_data.utc_offset_minutes) / 60;
                    if delta_hours != 0 {
                        let delta_str = if delta_hours > 0 {
                            format!("+{}h", delta_hours)
                        } else {
                            format!("{}h", delta_hours)
                        };
                        let delta_color = if delta_hours > 0 {
                            colors::DELTA_POSITIVE
                        } else {
                            colors::DELTA_NEGATIVE
                        };
                        draw.text(&delta_str)
                            .x_y(x + 120.0, item_y)
                            .color(delta_color)
                            .font_size(10);
                    }
                }
            }
        }
    }

    if display_order.len() > max_display {
        let more_str = format!("...and {} more", display_order.len() - max_display);
        draw.text(&more_str)
            .x_y(x, y - (max_display as f32) * item_height)
            .color(colors::ZONE_TEXT)
            .font_size(9);
    }
}

/// Draw the list view (accessibility mode)
pub fn draw_list_view(
    draw: &Draw,
    layout: &CoreLayout,
    display_order: &[Tz],
    zone_times: &HashMap<Tz, TimeData>,
    dominant_zone: Tz,
    compare_mode: bool,
) {
    let item_height = 50.0;
    let item_width = layout.width.min(500.0);
    let start_y = layout.center_y + ((display_order.len() as f32 - 1.0) * item_height) / 2.0;

    let dominant_data = zone_times.get(&dominant_zone);

    for (i, &tz) in display_order.iter().enumerate() {
        let item_y = start_y - (i as f32) * item_height;
        let is_dominant = tz == dominant_zone;

        if let Some(time_data) = zone_times.get(&tz) {
            // Background
            let bg_color = if is_dominant {
                colors::LIST_ITEM_DOMINANT
            } else {
                colors::LIST_ITEM_BG
            };
            draw.rect()
                .x_y(layout.center_x, item_y)
                .w_h(item_width, item_height - 4.0)
                .color(bg_color);

            // Border for dominant
            if is_dominant {
                draw.rect()
                    .x_y(layout.center_x, item_y)
                    .w_h(item_width, item_height - 4.0)
                    .no_fill()
                    .stroke(colors::CARD_BORDER_DOMINANT)
                    .stroke_weight(1.5);
            }

            // Zone name
            draw.text(tz.name())
                .x_y(layout.center_x - item_width * 0.35, item_y + 8.0)
                .color(if is_dominant {
                    colors::TIME_TEXT
                } else {
                    colors::ZONE_TEXT
                })
                .font_size(11)
                .left_justify();

            // Time
            let time_str = format!(
                "{}:{:02}:{:02} {}",
                time_data.hour12, time_data.minute, time_data.second, time_data.meridiem
            );
            draw.text(&time_str)
                .x_y(layout.center_x, item_y - 8.0)
                .color(colors::TIME_TEXT)
                .font_size(16);

            // Offset
            draw.text(&time_data.format_utc_offset())
                .x_y(layout.center_x + item_width * 0.35, item_y + 8.0)
                .color(colors::SECONDARY_TEXT)
                .font_size(10)
                .right_justify();

            // DST badge
            if time_data.is_dst {
                draw.text("DST")
                    .x_y(layout.center_x + item_width * 0.35, item_y - 8.0)
                    .color(colors::DST_ACTIVE)
                    .font_size(9)
                    .right_justify();
            }

            // Compare mode delta
            if compare_mode && !is_dominant {
                if let Some(dom_data) = dominant_data {
                    let delta_hours =
                        (time_data.utc_offset_minutes - dom_data.utc_offset_minutes) / 60;
                    if delta_hours != 0 {
                        let delta_str = if delta_hours > 0 {
                            format!("+{}h", delta_hours)
                        } else {
                            format!("{}h", delta_hours)
                        };
                        let delta_color = if delta_hours > 0 {
                            colors::DELTA_POSITIVE
                        } else {
                            colors::DELTA_NEGATIVE
                        };
                        draw.text(&delta_str)
                            .x_y(layout.center_x + item_width * 0.25, item_y)
                            .color(delta_color)
                            .font_size(12);
                    }
                }
            }

            // DST warning indicator
            let has_dst_warning = matches!(
                time_data.dst_change,
                DstChange::Upcoming { .. } | DstChange::JustOccurred { .. }
            );
            if has_dst_warning {
                draw.text("⚠")
                    .x_y(layout.center_x - item_width * 0.45, item_y)
                    .color(colors::DST_WARNING)
                    .font_size(14);
            }
        }
    }
}

/// Helper: compute day index relative to dominant zone
fn compute_day_index(zone_data: &TimeData, dominant_data: &TimeData) -> i32 {
    let zone_days = zone_data.year * 366 + zone_data.month as i32 * 31 + zone_data.day as i32;
    let dom_days =
        dominant_data.year * 366 + dominant_data.month as i32 * 31 + dominant_data.day as i32;
    (zone_days - dom_days).clamp(-1, 1)
}

/// Helper: get month abbreviation
fn month_abbrev(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

