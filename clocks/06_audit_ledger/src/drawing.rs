//! Drawing module for the Audit Ledger Clock
//!
//! Handles rendering of the terminal-style ledger view, header with
//! verification hash stamp, and all visual elements.

use nannou::prelude::*;
use shared::TimeData;

use crate::ledger::{BlockGroup, DstBadge, HourChapter};
use crate::Model;

/// Color palette for the terminal aesthetic
#[allow(dead_code)]
pub mod colors {
    use nannou::prelude::*;

    /// Deep background
    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 10,
        green: 12,
        blue: 16,
        standard: std::marker::PhantomData,
    };

    /// Phosphor green (primary text)
    pub const PHOSPHOR_GREEN: Srgb<u8> = Srgb {
        red: 51,
        green: 255,
        blue: 102,
        standard: std::marker::PhantomData,
    };

    /// Amber alternative
    pub const AMBER: Srgb<u8> = Srgb {
        red: 255,
        green: 179,
        blue: 71,
        standard: std::marker::PhantomData,
    };

    /// Dim text (secondary)
    pub const DIM_GREEN: Srgb<u8> = Srgb {
        red: 30,
        green: 120,
        blue: 60,
        standard: std::marker::PhantomData,
    };

    /// Header background
    pub const HEADER_BG: Srgb<u8> = Srgb {
        red: 15,
        green: 20,
        blue: 25,
        standard: std::marker::PhantomData,
    };

    /// Block header background
    pub const BLOCK_HEADER_BG: Srgb<u8> = Srgb {
        red: 20,
        green: 30,
        blue: 35,
        standard: std::marker::PhantomData,
    };

    /// Chapter header background (brighter than block)
    pub const CHAPTER_HEADER_BG: Srgb<u8> = Srgb {
        red: 25,
        green: 40,
        blue: 50,
        standard: std::marker::PhantomData,
    };

    /// Chapter header accent color
    pub const CHAPTER_ACCENT: Srgb<u8> = Srgb {
        red: 80,
        green: 200,
        blue: 120,
        standard: std::marker::PhantomData,
    };

    /// DST warning color
    pub const DST_WARNING: Srgb<u8> = Srgb {
        red: 255,
        green: 150,
        blue: 80,
        standard: std::marker::PhantomData,
    };

    /// DST active badge
    pub const DST_ACTIVE: Srgb<u8> = Srgb {
        red: 255,
        green: 200,
        blue: 100,
        standard: std::marker::PhantomData,
    };

    /// Gap marker color
    pub const GAP_MARKER: Srgb<u8> = Srgb {
        red: 255,
        green: 100,
        blue: 100,
        standard: std::marker::PhantomData,
    };

    /// Focus ring color
    pub const FOCUS_RING: Srgb<u8> = Srgb {
        red: 100,
        green: 200,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Separator line color
    pub const SEPARATOR: Srgb<u8> = Srgb {
        red: 40,
        green: 50,
        blue: 55,
        standard: std::marker::PhantomData,
    };

    /// Return to live button
    pub const LIVE_BUTTON: Srgb<u8> = Srgb {
        red: 255,
        green: 80,
        blue: 80,
        standard: std::marker::PhantomData,
    };

    /// Hash stamp color
    pub const HASH_COLOR: Srgb<u8> = Srgb {
        red: 80,
        green: 180,
        blue: 255,
        standard: std::marker::PhantomData,
    };

    /// Overlay background with alpha
    pub fn overlay_bg() -> Srgba<u8> {
        srgba(10, 12, 16, 230)
    }
}

/// Draw the header with time display and verification hash
pub fn draw_header(draw: &Draw, rect: &Rect, time_data: &TimeData, hash: &str) {
    let header_height = 70.0;
    let header_y = rect.top() - header_height / 2.0;

    // Header background
    draw.rect()
        .x_y(rect.x(), header_y)
        .w_h(rect.w(), header_height)
        .color(colors::HEADER_BG);

    // ASCII border
    let border_y = rect.top() - header_height;
    draw.text(&"═".repeat(60))
        .x_y(rect.x(), border_y)
        .color(colors::DIM_GREEN)
        .font_size(14)
        .w(rect.w());

    // Title
    draw.text("╔══ AUDIT LEDGER CLOCK ══╗")
        .x_y(rect.x(), rect.top() - 20.0)
        .color(colors::PHOSPHOR_GREEN)
        .font_size(16)
        .w(400.0);

    // Current time display
    let time_str = format!(
        "{:02}:{:02}:{:02} {} │ {} │ {}",
        time_data.hour12,
        time_data.minute,
        time_data.second,
        time_data.meridiem,
        time_data.tz_abbrev,
        time_data.format_utc_offset()
    );
    draw.text(&time_str)
        .x_y(rect.x() - 100.0, rect.top() - 45.0)
        .color(colors::PHOSPHOR_GREEN)
        .font_size(18)
        .w(400.0);

    // Verification hash stamp
    draw.text(&format!("HASH: {}", hash))
        .x_y(rect.x() + 180.0, rect.top() - 45.0)
        .color(colors::HASH_COLOR)
        .font_size(14)
        .w(200.0);

    // DST indicator in header
    if time_data.is_dst {
        draw.text("● DST")
            .x_y(rect.x() + 300.0, rect.top() - 45.0)
            .color(colors::DST_ACTIVE)
            .font_size(12);
    }
}

/// Draw the ledger view with hierarchical chapter -> block structure
pub fn draw_ledger(draw: &Draw, rect: &Rect, model: &Model) {
    let chapters = model.ledger.get_chapter_grouped_entries();
    let chapter_header_height = 36.0;
    let block_header_height = 26.0;
    let row_height = model.text_density.row_height();
    let font_size = model.text_density.font_size();

    // Start below the main header (70px) + column headers (30px) + padding
    let content_top = rect.top() - 125.0;
    let mut current_y = content_top - model.ledger.scroll_offset;

    // Relabel animation progress (for sweep effect)
    let relabel_progress = if model.relabel_start.is_some() {
        model.relabel_progress
    } else {
        1.0
    };

    // Track indices for focus
    let mut global_block_idx = 0;

    for chapter in chapters.iter() {
        // Calculate chapter height for viewport culling
        let chapter_content_height = if chapter.collapsed {
            0.0
        } else {
            chapter.blocks.iter().map(|b| {
                if b.collapsed {
                    block_header_height
                } else {
                    block_header_height + row_height * b.entries.len() as f32
                }
            }).sum::<f32>()
        };

        // Skip if completely above viewport
        if current_y > content_top + 50.0 {
            current_y -= chapter_header_height + chapter_content_height;
            global_block_idx += chapter.blocks.len();
            continue;
        }

        // Skip if below viewport
        if current_y < rect.bottom() - 100.0 {
            break;
        }

        // Draw chapter header
        let is_chapter_focused = model.focused_block_index.map_or(false, |idx| {
            idx >= global_block_idx && idx < global_block_idx + chapter.blocks.len()
        });
        draw_chapter_header(draw, rect.x(), current_y, rect.w() - 40.0, chapter, is_chapter_focused);
        current_y -= chapter_header_height;

        // Draw blocks if chapter not collapsed
        if !chapter.collapsed {
            for block in chapter.blocks.iter() {
                if current_y < rect.bottom() - 50.0 {
                    break;
                }

                // Draw block header (indented)
                let is_block_focused = model.focused_block_index == Some(global_block_idx);
                draw_block_header(draw, rect.x() + 20.0, current_y, rect.w() - 60.0, block, is_block_focused);
                current_y -= block_header_height;

                // Draw entries if block not collapsed
                if !block.collapsed {
                    for (entry_idx, entry) in block.entries.iter().enumerate() {
                        if current_y < rect.bottom() - 50.0 {
                            break;
                        }

                        // Calculate animation alpha for sweep effect
                        let entry_progress = (entry_idx as f32) / (block.entries.len().max(1) as f32);
                        let alpha = if relabel_progress < 1.0 {
                            if entry_progress < relabel_progress {
                                1.0
                            } else {
                                0.3
                            }
                        } else {
                            1.0
                        };

                        draw_ledger_row(draw, rect.x() + 20.0, current_y, rect.w() - 60.0, entry, font_size, alpha);
                        current_y -= row_height;
                    }
                }

                global_block_idx += 1;
            }
        } else {
            global_block_idx += chapter.blocks.len();
        }
    }

    // Draw column headers at fixed position (below main header, above ledger content)
    let column_headers_y = rect.top() - 90.0;
    draw_column_headers(draw, rect.x(), column_headers_y, rect.w() - 40.0);
}

/// Draw column headers
fn draw_column_headers(draw: &Draw, x: f32, y: f32, width: f32) {
    let header_text = "│ TIMESTAMP    │ BLK │ CH │ OFFSET     │ DST │";

    draw.rect()
        .x_y(x, y)
        .w_h(width, 20.0)
        .color(colors::HEADER_BG);

    draw.text(header_text)
        .x_y(x, y)
        .color(colors::DIM_GREEN)
        .font_size(12)
        .w(width);

    // Separator
    draw.text(&"─".repeat(50))
        .x_y(x, y - 12.0)
        .color(colors::SEPARATOR)
        .font_size(12);
}

/// Draw an hour chapter header
fn draw_chapter_header(draw: &Draw, x: f32, y: f32, width: f32, chapter: &HourChapter, is_focused: bool) {
    // Background
    let bg_color = if is_focused {
        colors::CHAPTER_HEADER_BG
    } else {
        srgb(22, 35, 42)
    };

    draw.rect()
        .x_y(x, y)
        .w_h(width, 34.0)
        .color(bg_color);

    // Left accent bar
    draw.rect()
        .x_y(x - width / 2.0 + 3.0, y)
        .w_h(4.0, 30.0)
        .color(colors::CHAPTER_ACCENT);

    // Collapse indicator
    let collapse_char = if chapter.collapsed { "▶" } else { "▼" };

    // Format hour for display
    let (hour_12, meridiem) = chapter.hour_12();
    let total_entries: usize = chapter.blocks.iter().map(|b| b.entries.len()).sum();

    // Header text
    let header_text = format!(
        "{} ═══ CHAPTER {:02} ({:02} {}) ═══ {} blocks │ {} entries",
        collapse_char,
        chapter.hour,
        hour_12,
        meridiem,
        chapter.blocks.len(),
        total_entries
    );

    draw.text(&header_text)
        .x_y(x + 10.0, y)
        .color(colors::CHAPTER_ACCENT)
        .font_size(14)
        .w(width - 20.0);

    // Focus ring
    if is_focused {
        draw.rect()
            .x_y(x, y)
            .w_h(width + 4.0, 38.0)
            .no_fill()
            .stroke(colors::FOCUS_RING)
            .stroke_weight(2.0);
    }
}

/// Draw a block header
fn draw_block_header(draw: &Draw, x: f32, y: f32, width: f32, group: &BlockGroup, is_focused: bool) {
    // Background
    let bg_color = if is_focused {
        colors::BLOCK_HEADER_BG
    } else {
        srgb(18, 25, 30)
    };

    draw.rect()
        .x_y(x, y)
        .w_h(width, 24.0)
        .color(bg_color);

    // Collapse indicator
    let collapse_char = if group.collapsed { "▸" } else { "▾" };

    // Header text (simplified - chapter info is in parent)
    let header_text = format!(
        "{} BLOCK {:02} │ {} entries",
        collapse_char,
        group.minute,
        group.entries.len()
    );

    draw.text(&header_text)
        .x_y(x, y)
        .color(colors::PHOSPHOR_GREEN)
        .font_size(12)
        .w(width);

    // Focus ring
    if is_focused {
        draw.rect()
            .x_y(x, y)
            .w_h(width + 4.0, 28.0)
            .no_fill()
            .stroke(colors::FOCUS_RING)
            .stroke_weight(2.0);
    }
}

/// Draw a single ledger row
fn draw_ledger_row(
    draw: &Draw,
    x: f32,
    y: f32,
    width: f32,
    entry: &crate::ledger::LedgerEntry,
    font_size: u32,
    alpha: f32,
) {
    // Determine row color based on entry type
    let (text_color, is_special) = match &entry.dst_badge {
        DstBadge::GapMarker { .. } => (colors::GAP_MARKER, true),
        DstBadge::OverlapPass1 | DstBadge::OverlapPass2 => (colors::DST_WARNING, true),
        _ => (colors::PHOSPHOR_GREEN, false),
    };

    // Apply alpha
    let text_color = srgba(
        text_color.red,
        text_color.green,
        text_color.blue,
        (255.0 * alpha) as u8,
    );

    // Special marker row
    if is_special {
        match &entry.dst_badge {
            DstBadge::GapMarker { from, to } => {
                let marker_text = format!("│ ══ DST GAP: {} → {} ══ │", from, to);
                draw.text(&marker_text)
                    .x_y(x, y)
                    .color(text_color)
                    .font_size(font_size)
                    .w(width);
                return;
            }
            DstBadge::OverlapPass1 => {
                draw_normal_row(draw, x, y, width, entry, font_size, text_color, "P1");
                return;
            }
            DstBadge::OverlapPass2 => {
                draw_normal_row(draw, x, y, width, entry, font_size, text_color, "P2");
                return;
            }
            _ => {}
        }
    }

    // Normal row
    let dst_str = match &entry.dst_badge {
        DstBadge::Active => "DST",
        _ => "   ",
    };
    draw_normal_row(draw, x, y, width, entry, font_size, text_color, dst_str);
}

/// Draw a normal ledger row with all columns
fn draw_normal_row(
    draw: &Draw,
    x: f32,
    y: f32,
    width: f32,
    entry: &crate::ledger::LedgerEntry,
    font_size: u32,
    color: Srgba<u8>,
    dst_str: &str,
) {
    let row_text = format!(
        "│ {} │ {:02}  │ {:02} │ {} │ {} │",
        entry.local_timestamp,
        entry.block_id,
        entry.chapter_id,
        entry.offset_str,
        dst_str
    );

    draw.text(&row_text)
        .x_y(x, y)
        .color(color)
        .font_size(font_size)
        .w(width);
}

/// Draw the "Return to Live" button
pub fn draw_return_to_live_button(draw: &Draw, rect: &Rect) {
    let button_x = rect.x();
    let button_y = rect.bottom() + 60.0;
    let button_w = 200.0;
    let button_h = 40.0;

    // Button background
    draw.rect()
        .x_y(button_x, button_y)
        .w_h(button_w, button_h)
        .color(srgba(colors::LIVE_BUTTON.red, colors::LIVE_BUTTON.green, colors::LIVE_BUTTON.blue, 200));

    // Button border
    draw.rect()
        .x_y(button_x, button_y)
        .w_h(button_w, button_h)
        .no_fill()
        .stroke(colors::LIVE_BUTTON)
        .stroke_weight(2.0);

    // Button text
    draw.text("▲ RETURN TO LIVE (L)")
        .x_y(button_x, button_y)
        .color(srgb(255u8, 255u8, 255u8))
        .font_size(14);
}

/// Draw error banner for TZ data issues
pub fn draw_error_banner(draw: &Draw, window_rect: Rect) {
    let banner_height = 40.0;
    let banner_y = window_rect.top() - 20.0;

    // Background
    draw.rect()
        .x_y(0.0, banner_y)
        .w_h(window_rect.w(), banner_height)
        .color(srgba(120u8, 40u8, 40u8, 220u8));

    // Text
    draw.text("⚠ Timezone data may be missing or stale. Showing UTC as fallback.")
        .x_y(0.0, banner_y)
        .color(srgb(255u8, 255u8, 255u8))
        .font_size(14)
        .w(window_rect.w() - 40.0);
}

/// Draw toast notifications
pub fn draw_toasts(draw: &Draw, window_rect: Rect, toasts: &[crate::Toast]) {
    let toast_width = 350.0;
    let toast_height = 40.0;
    let padding = 10.0;
    let start_y = window_rect.bottom() + 80.0;

    for (i, toast) in toasts.iter().enumerate() {
        let y = start_y + (i as f32) * (toast_height + padding);
        let alpha = (toast.alpha() * 220.0) as u8;

        // Background
        draw.rect()
            .x_y(0.0, y)
            .w_h(toast_width, toast_height)
            .color(srgba(30u8, 35u8, 40u8, alpha));

        // Border
        draw.rect()
            .x_y(0.0, y)
            .w_h(toast_width, toast_height)
            .no_fill()
            .stroke(srgba(colors::PHOSPHOR_GREEN.red, colors::PHOSPHOR_GREEN.green, colors::PHOSPHOR_GREEN.blue, alpha))
            .stroke_weight(1.0);

        // Text
        let text_alpha = (toast.alpha() * 255.0) as u8;
        draw.text(&toast.message)
            .x_y(0.0, y)
            .color(srgba(colors::PHOSPHOR_GREEN.red, colors::PHOSPHOR_GREEN.green, colors::PHOSPHOR_GREEN.blue, text_alpha))
            .font_size(12)
            .w(toast_width - 20.0);
    }
}

/// Draw focus indicator for ledger region
pub fn draw_focus_indicator(draw: &Draw, rect: &Rect) {
    draw.rect()
        .x_y(rect.x(), rect.y())
        .w_h(rect.w() + 4.0, rect.h() + 4.0)
        .no_fill()
        .stroke(srgba(colors::FOCUS_RING.red, colors::FOCUS_RING.green, colors::FOCUS_RING.blue, 100))
        .stroke_weight(2.0);
}

