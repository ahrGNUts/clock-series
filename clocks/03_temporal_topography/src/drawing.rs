//! Drawing module - DayMapCanvas, terrain rendering, grid, and locator beacon
//!
//! Renders the topographic day map with its cartographic aesthetic.

use nannou::prelude::*;

use crate::terrain::{DayDomain, HourBoundary, TerrainParams, terrain_elevation};

/// Color palette for the temporal topography theme - cartographic/topographic aesthetic
#[allow(dead_code)]
pub mod colors {
    use nannou::prelude::*;

    /// Deep slate background (paper tone)
    pub const BACKGROUND: Srgb<u8> = Srgb {
        red: 28,
        green: 32,
        blue: 36,
        standard: std::marker::PhantomData,
    };

    /// Map canvas background
    pub const CANVAS_BG: Srgb<u8> = Srgb {
        red: 38,
        green: 42,
        blue: 48,
        standard: std::marker::PhantomData,
    };

    /// Terrain peak color (warm brown/tan)
    pub const TERRAIN_PEAK: Srgb<u8> = Srgb {
        red: 139,
        green: 119,
        blue: 101,
        standard: std::marker::PhantomData,
    };

    /// Terrain valley color (cool blue-green)
    pub const TERRAIN_VALLEY: Srgb<u8> = Srgb {
        red: 70,
        green: 100,
        blue: 90,
        standard: std::marker::PhantomData,
    };

    /// Terrain mid color (neutral)
    pub const TERRAIN_MID: Srgb<u8> = Srgb {
        red: 90,
        green: 95,
        blue: 88,
        standard: std::marker::PhantomData,
    };

    /// Terrain fill (under the curve)
    pub const TERRAIN_FILL: Srgb<u8> = Srgb {
        red: 50,
        green: 58,
        blue: 55,
        standard: std::marker::PhantomData,
    };

    /// Grid line - major (hours)
    pub const GRID_MAJOR: Srgb<u8> = Srgb {
        red: 100,
        green: 100,
        blue: 100,
        standard: std::marker::PhantomData,
    };

    /// Grid line - minor (15 min)
    pub const GRID_MINOR: Srgb<u8> = Srgb {
        red: 60,
        green: 60,
        blue: 60,
        standard: std::marker::PhantomData,
    };

    /// Locator beacon - bright amber/gold
    pub const BEACON: Srgb<u8> = Srgb {
        red: 255,
        green: 179,
        blue: 71,
        standard: std::marker::PhantomData,
    };

    /// Inspect mode color
    pub const INSPECT: Srgb<u8> = Srgb {
        red: 120,
        green: 180,
        blue: 220,
        standard: std::marker::PhantomData,
    };

    /// DST fault line - warning orange
    pub const DST_FAULT: Srgb<u8> = Srgb {
        red: 255,
        green: 107,
        blue: 53,
        standard: std::marker::PhantomData,
    };

    /// Text primary
    pub const TEXT_PRIMARY: Srgb<u8> = Srgb {
        red: 220,
        green: 215,
        blue: 210,
        standard: std::marker::PhantomData,
    };

    /// Text secondary
    pub const TEXT_SECONDARY: Srgb<u8> = Srgb {
        red: 140,
        green: 135,
        blue: 130,
        standard: std::marker::PhantomData,
    };

    /// Midnight marker
    pub const MIDNIGHT: Srgb<u8> = Srgb {
        red: 180,
        green: 160,
        blue: 220,
        standard: std::marker::PhantomData,
    };
}

/// Layout configuration for the day map canvas
#[derive(Debug, Clone)]
pub struct MapLayout {
    /// Left edge of the map canvas (x coordinate)
    pub left: f32,
    /// Right edge of the map canvas (x coordinate)
    pub right: f32,
    /// Top edge of the map canvas (y coordinate)
    pub top: f32,
    /// Bottom edge of the map canvas (y coordinate)
    pub bottom: f32,
    /// Width of the map canvas
    pub width: f32,
    /// Height of the map canvas
    pub height: f32,
    /// Center Y position (where elevation 0 is drawn)
    pub center_y: f32,
    /// Amplitude for elevation mapping (max displacement from center)
    pub amplitude: f32,
    /// Number of terrain samples
    pub sample_count: usize,
}

impl MapLayout {
    /// Calculate layout from window dimensions, accounting for side panel
    pub fn calculate(window_rect: Rect, side_panel_width: f32) -> Self {
        let margin = 40.0;
        let left = window_rect.left() + margin;
        let right = window_rect.right() - side_panel_width - margin;
        let top = window_rect.top() - margin - 60.0; // Leave room for title
        let bottom = window_rect.bottom() + margin + 40.0; // Leave room for labels

        let width = right - left;
        let height = top - bottom;
        let center_y = (top + bottom) / 2.0;
        let amplitude = height * 0.35;

        // Sample count based on width
        let sample_count = (width as usize).max(240);

        Self {
            left,
            right,
            top,
            bottom,
            width,
            height,
            center_y,
            amplitude,
            sample_count,
        }
    }

    /// Convert normalized position [0..1] to x coordinate
    pub fn position_to_x(&self, p: f32) -> f32 {
        self.left + p * self.width
    }

    /// Convert x coordinate to normalized position [0..1]
    pub fn x_to_position(&self, x: f32) -> f32 {
        ((x - self.left) / self.width).clamp(0.0, 1.0)
    }

    /// Convert elevation [-1..1] to y coordinate
    pub fn elevation_to_y(&self, e: f32) -> f32 {
        self.center_y + e * self.amplitude
    }

    /// Check if a point is within the map canvas
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.bottom && y <= self.top
    }
}

/// Draw the complete day map visualization
pub fn draw_day_map(
    draw: &Draw,
    layout: &MapLayout,
    params: &TerrainParams,
    day_domain: &DayDomain,
    hour_boundaries: &[HourBoundary],
    reduced_motion: bool,
    time_fraction: f32,
) {
    // Draw canvas background
    draw_canvas_background(draw, layout);

    // Draw grid layer (behind terrain)
    draw_grid_layer(draw, layout, hour_boundaries, day_domain);

    // Draw DST fault lines (behind terrain but on top of grid)
    draw_fault_lines(draw, layout, day_domain);

    // Draw terrain layer
    draw_terrain_layer(draw, layout, params, day_domain);

    // Draw dual traces for any DST fall-back overlaps
    for fault in &day_domain.dst_faults {
        if fault.delta_minutes < 0 {
            // Fall back - draw dual traces
            let fault_x = layout.position_to_x(fault.position);
            let fault_width_px = fault.width * layout.width;
            draw_overlap_dual_traces(draw, layout, params, fault_x, fault_width_px);
        }
    }

    // Draw locator beacon
    draw_locator_beacon(draw, layout, params, day_domain, reduced_motion, time_fraction);
}

/// Draw the canvas background
fn draw_canvas_background(draw: &Draw, layout: &MapLayout) {
    draw.rect()
        .x_y((layout.left + layout.right) / 2.0, layout.center_y)
        .w_h(layout.width + 20.0, layout.height + 20.0)
        .color(colors::CANVAS_BG);

    // Subtle border
    let half_w = layout.width / 2.0 + 10.0;
    let half_h = layout.height / 2.0 + 10.0;
    let center_x = (layout.left + layout.right) / 2.0;

    draw.line()
        .start(pt2(center_x - half_w, layout.center_y + half_h))
        .end(pt2(center_x + half_w, layout.center_y + half_h))
        .color(colors::GRID_MINOR)
        .weight(1.0);

    draw.line()
        .start(pt2(center_x - half_w, layout.center_y - half_h))
        .end(pt2(center_x + half_w, layout.center_y - half_h))
        .color(colors::GRID_MINOR)
        .weight(1.0);
}

/// Draw the grid layer with hour and 15-minute marks
fn draw_grid_layer(
    draw: &Draw,
    layout: &MapLayout,
    hour_boundaries: &[HourBoundary],
    day_domain: &DayDomain,
) {
    // Draw 15-minute marks first (behind hour lines)
    for i in 0..96 {
        let ssm = i * 900; // 15 minutes = 900 seconds
        if ssm > day_domain.day_length_seconds {
            break;
        }

        // Skip if it's an hour boundary
        if i % 4 == 0 {
            continue;
        }

        let p = day_domain.ssm_to_position(ssm);
        let x = layout.position_to_x(p);

        draw.line()
            .start(pt2(x, layout.top - 5.0))
            .end(pt2(x, layout.bottom + 5.0))
            .color(srgba(60u8, 60u8, 60u8, 80u8))
            .weight(0.5);
    }

    // Draw hour boundaries
    for boundary in hour_boundaries {
        let x = layout.position_to_x(boundary.position);

        let (color, weight) = if boundary.is_midnight {
            (colors::MIDNIGHT, 2.0)
        } else {
            (colors::GRID_MAJOR, 1.0)
        };

        draw.line()
            .start(pt2(x, layout.top))
            .end(pt2(x, layout.bottom))
            .color(color)
            .weight(weight);

        // Draw label below the grid
        let label_y = layout.bottom - 15.0;
        let label = if let Some(ref suffix) = boundary.suffix {
            format!("{} {}", boundary.label, suffix)
        } else {
            boundary.label.clone()
        };

        let label_color = if boundary.is_midnight {
            colors::MIDNIGHT
        } else {
            colors::TEXT_SECONDARY
        };

        draw.text(&label)
            .x_y(x, label_y)
            .color(label_color)
            .font_size(10)
            .w(60.0);
        
        // Draw "(next)" below for next day's midnight
        if boundary.is_next_day {
            draw.text("(next)")
                .x_y(x, label_y - 12.0)
                .color(label_color)
                .font_size(9)
                .w(60.0);
        }
    }
}

/// Draw DST fault lines
fn draw_fault_lines(draw: &Draw, layout: &MapLayout, day_domain: &DayDomain) {
    for fault in &day_domain.dst_faults {
        let x = layout.position_to_x(fault.position);
        let fault_width_px = fault.width * layout.width;

        if fault.delta_minutes > 0 {
            // Spring forward - gap
            // Draw dashed vertical lines at edges
            draw_dashed_line(
                draw,
                pt2(x, layout.top),
                pt2(x, layout.bottom),
                colors::DST_FAULT,
                2.0,
                8.0,
                4.0,
            );

            draw_dashed_line(
                draw,
                pt2(x + fault_width_px, layout.top),
                pt2(x + fault_width_px, layout.bottom),
                colors::DST_FAULT,
                2.0,
                8.0,
                4.0,
            );

            // Fill gap with semi-transparent overlay
            draw.rect()
                .x_y(x + fault_width_px / 2.0, layout.center_y)
                .w_h(fault_width_px, layout.height)
                .color(srgba(255u8, 107u8, 53u8, 30u8));

            // Label
            draw.text("GAP")
                .x_y(x + fault_width_px / 2.0, layout.top + 15.0)
                .color(colors::DST_FAULT)
                .font_size(10)
                .w(60.0);
        } else {
            // Fall back - overlap
            // Draw solid line at the overlap position
            draw.line()
                .start(pt2(x, layout.top))
                .end(pt2(x, layout.bottom))
                .color(colors::DST_FAULT)
                .weight(2.0);

            // Fill overlap region with subtle overlay
            draw.rect()
                .x_y(x + fault_width_px / 2.0, layout.center_y)
                .w_h(fault_width_px, layout.height)
                .color(srgba(255u8, 179u8, 71u8, 20u8));

            // Labels for A and B
            if let Some(ref label_a) = fault.label_a {
                draw.text(label_a)
                    .x_y(x + fault_width_px * 0.25, layout.top + 15.0)
                    .color(colors::DST_FAULT)
                    .font_size(10)
                    .w(30.0);
            }
            if let Some(ref label_b) = fault.label_b {
                draw.text(label_b)
                    .x_y(x + fault_width_px * 0.75, layout.top + 15.0)
                    .color(colors::DST_FAULT)
                    .font_size(10)
                    .w(30.0);
            }
            
            // Draw label for repeated hour
            draw.text("Repeated Hour")
                .x_y(x + fault_width_px / 2.0, layout.bottom - 30.0)
                .color(srgba(255u8, 179u8, 71u8, 180u8))
                .font_size(9)
                .w(80.0);
        }
    }
}

/// Draw dual terrain traces for DST fall-back overlap region
/// This is called from draw_terrain_layer when an overlap is detected
pub fn draw_overlap_dual_traces(
    draw: &Draw,
    layout: &MapLayout,
    params: &TerrainParams,
    fault_start_x: f32,
    fault_width_px: f32,
) {
    let sample_count = (fault_width_px as usize).max(20);
    
    // Trace A - slightly offset up (before transition)
    let mut trace_a_points: Vec<Point2> = Vec::with_capacity(sample_count);
    // Trace B - slightly offset down (after transition)  
    let mut trace_b_points: Vec<Point2> = Vec::with_capacity(sample_count);
    
    let y_offset = 8.0; // Vertical offset between traces
    
    for i in 0..sample_count {
        let t = i as f32 / (sample_count - 1) as f32;
        let sample_x = fault_start_x + t * fault_width_px;
        let p = layout.x_to_position(sample_x);
        
        let elevation = terrain_elevation(p, params);
        let base_y = layout.elevation_to_y(elevation);
        
        trace_a_points.push(pt2(sample_x, base_y + y_offset));
        trace_b_points.push(pt2(sample_x, base_y - y_offset));
    }
    
    // Draw Trace A (faint, warmer color)
    for i in 0..trace_a_points.len().saturating_sub(1) {
        draw.line()
            .start(trace_a_points[i])
            .end(trace_a_points[i + 1])
            .color(srgba(255u8, 179u8, 71u8, 100u8))
            .weight(1.5);
    }
    
    // Draw Trace B (faint, cooler color)
    for i in 0..trace_b_points.len().saturating_sub(1) {
        draw.line()
            .start(trace_b_points[i])
            .end(trace_b_points[i + 1])
            .color(srgba(120u8, 180u8, 220u8, 100u8))
            .weight(1.5);
    }
}

/// Draw a dashed line
fn draw_dashed_line(
    draw: &Draw,
    start: Point2,
    end: Point2,
    color: Srgb<u8>,
    weight: f32,
    dash_length: f32,
    gap_length: f32,
) {
    let dir = end - start;
    let length = dir.length();
    let dir_norm = dir / length;

    let mut pos = 0.0;
    let mut drawing = true;

    while pos < length {
        let segment_length = if drawing { dash_length } else { gap_length };
        let segment_end = (pos + segment_length).min(length);

        if drawing {
            let p1 = start + dir_norm * pos;
            let p2 = start + dir_norm * segment_end;
            draw.line()
                .start(p1)
                .end(p2)
                .color(color)
                .weight(weight);
        }

        pos = segment_end;
        drawing = !drawing;
    }
}

/// Draw the terrain layer
fn draw_terrain_layer(
    draw: &Draw,
    layout: &MapLayout,
    params: &TerrainParams,
    day_domain: &DayDomain,
) {
    let sample_count = layout.sample_count;
    let mut points: Vec<Point2> = Vec::with_capacity(sample_count + 2);
    let mut fill_points: Vec<Point2> = Vec::with_capacity(sample_count + 2);

    // Start fill at bottom-left
    fill_points.push(pt2(layout.left, layout.bottom));

    for i in 0..sample_count {
        let p = i as f32 / (sample_count - 1) as f32;
        let x = layout.position_to_x(p);

        // Check if in DST gap
        let in_gap = day_domain.is_in_gap(p);

        if in_gap {
            // Break the line at gaps - draw what we have and start fresh
            if points.len() > 1 {
                draw_terrain_segment(draw, &points, layout);
            }
            points.clear();

            // Continue fill at bottom
            fill_points.push(pt2(x, layout.bottom));
        } else {
            let elevation = terrain_elevation(p, params);
            let y = layout.elevation_to_y(elevation);

            points.push(pt2(x, y));
            fill_points.push(pt2(x, y));
        }
    }

    // Draw remaining terrain segment
    if points.len() > 1 {
        draw_terrain_segment(draw, &points, layout);
    }

    // Close fill polygon at bottom-right
    fill_points.push(pt2(layout.right, layout.bottom));

    // Draw fill (under the terrain curve)
    if fill_points.len() > 2 {
        draw.polygon()
            .points(fill_points)
            .color(srgba(50u8, 58u8, 55u8, 100u8));
    }
}

/// Draw a terrain segment with color gradient based on elevation
fn draw_terrain_segment(draw: &Draw, points: &[Point2], layout: &MapLayout) {
    if points.len() < 2 {
        return;
    }

    // Draw line segments with color based on elevation
    for i in 0..points.len() - 1 {
        let p1 = points[i];
        let p2 = points[i + 1];

        // Calculate average elevation for this segment
        let avg_y = (p1.y + p2.y) / 2.0;
        let normalized_elevation = (avg_y - layout.center_y) / layout.amplitude;

        // Interpolate color based on elevation
        let color = elevation_color(normalized_elevation);

        draw.line()
            .start(p1)
            .end(p2)
            .color(color)
            .weight(2.5);
    }
}

/// Get color for a given normalized elevation [-1..1]
fn elevation_color(e: f32) -> Srgba<u8> {
    if e > 0.0 {
        // Peak colors (brown/tan)
        let t = e.clamp(0.0, 1.0);
        let r = lerp(90.0, 139.0, t) as u8;
        let g = lerp(95.0, 119.0, t) as u8;
        let b = lerp(88.0, 101.0, t) as u8;
        srgba(r, g, b, 255u8)
    } else {
        // Valley colors (blue-green)
        let t = (-e).clamp(0.0, 1.0);
        let r = lerp(90.0, 70.0, t) as u8;
        let g = lerp(95.0, 100.0, t) as u8;
        let b = lerp(88.0, 90.0, t) as u8;
        srgba(r, g, b, 255u8)
    }
}

/// Linear interpolation helper
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Draw the locator beacon at the current time position
fn draw_locator_beacon(
    draw: &Draw,
    layout: &MapLayout,
    params: &TerrainParams,
    day_domain: &DayDomain,
    reduced_motion: bool,
    time_fraction: f32,
) {
    let p = day_domain.normalized_position;
    let x = layout.position_to_x(p);

    // Get terrain height at this position - beacon follows the terrain line
    let elevation = terrain_elevation(p, params);
    let y = layout.elevation_to_y(elevation);

    let beacon_size = 12.0;

    if reduced_motion {
        // Reduced motion: toggle outline for 200ms at second boundary
        let show_outline = time_fraction < 0.2; // First 200ms of each second
        
        if show_outline {
            // Draw outline only
            draw.ellipse()
                .x_y(x, y)
                .w_h(beacon_size, beacon_size)
                .no_fill()
                .stroke(colors::BEACON)
                .stroke_weight(2.0);
        } else {
            // Draw filled
            draw.ellipse()
                .x_y(x, y)
                .w_h(beacon_size, beacon_size)
                .color(colors::BEACON);
        }
    } else {
        // Pulse animation
        let pulse_t = (time_fraction / 0.35).min(1.0);
        let ease = pulse_t * (2.0 - pulse_t); // ease out
        let pulse_scale = 1.0 + 0.4 * (1.0 - ease).max(0.0);
        let animated_size = beacon_size * pulse_scale;

        // Glow effect
        for i in 1..=4 {
            let glow_size = animated_size + i as f32 * 4.0;
            let alpha = (60 - i * 12) as u8;
            draw.ellipse()
                .x_y(x, y)
                .w_h(glow_size, glow_size)
                .color(srgba(255u8, 179u8, 71u8, alpha));
        }

        // Main beacon dot
        draw.ellipse()
            .x_y(x, y)
            .w_h(animated_size, animated_size)
            .color(colors::BEACON);
    }

    // Vertical line through beacon
    draw.line()
        .start(pt2(x, layout.top))
        .end(pt2(x, layout.bottom))
        .color(srgba(255u8, 179u8, 71u8, 80u8))
        .weight(1.0);

    // Label above
    draw.text("NOW")
        .x_y(x, layout.top + 25.0)
        .color(colors::BEACON)
        .font_size(12)
        .w(40.0);
}

/// Draw the inspect cursor at a given position
pub fn draw_inspect_cursor(
    draw: &Draw,
    layout: &MapLayout,
    position: f32,
    is_pinned: bool,
) {
    let x = layout.position_to_x(position);

    let color = if is_pinned {
        srgba(120u8, 180u8, 220u8, 255u8)
    } else {
        srgba(120u8, 180u8, 220u8, 180u8)
    };

    // Vertical line
    draw.line()
        .start(pt2(x, layout.top))
        .end(pt2(x, layout.bottom))
        .color(color)
        .weight(2.0);

    // Cursor triangle at top
    let tri_size = 8.0;
    let points = vec![
        pt2(x, layout.top),
        pt2(x - tri_size, layout.top + tri_size),
        pt2(x + tri_size, layout.top + tri_size),
    ];
    draw.polygon()
        .points(points)
        .color(color);

    // Pin indicator
    if is_pinned {
        draw.text("📌")
            .x_y(x, layout.top + 25.0)
            .font_size(14)
            .w(30.0);
    }
}

/// Draw the title and map summary
pub fn draw_title(draw: &Draw, window_rect: Rect) {
    let title_y = window_rect.top() - 30.0;

    draw.text("Temporal Topography")
        .x_y(window_rect.left() + 150.0, title_y)
        .color(colors::TEXT_PRIMARY)
        .font_size(20)
        .w(300.0);
}

/// Draw keyboard help hints at the bottom
pub fn draw_help_hints(draw: &Draw, layout: &MapLayout, window_rect: Rect) {
    let help_y = window_rect.bottom() + 15.0;
    // Center within the map canvas, not the whole window
    let center_x = (layout.left + layout.right) / 2.0;

    draw.text("Click map to inspect  •  ←/→ step minute  •  Shift+←/→ step hour  •  Esc return to now  •  / search timezone")
        .x_y(center_x, help_y)
        .color(srgba(140u8, 135u8, 130u8, 150u8))
        .font_size(10)
        .w(layout.width);
}

/// Draw hover tooltip showing time at cursor position
pub fn draw_hover_tooltip(
    draw: &Draw,
    layout: &MapLayout,
    mouse_x: f32,
    mouse_y: f32,
    time_str: &str,
) {
    // Only draw if mouse is within the map canvas
    if !layout.contains(mouse_x, mouse_y) {
        return;
    }

    // Position tooltip above the cursor
    let tooltip_x = mouse_x;
    let tooltip_y = mouse_y + 25.0;

    // Background box
    let padding = 8.0;
    let text_width = 80.0;
    let text_height = 16.0;

    draw.rect()
        .x_y(tooltip_x, tooltip_y)
        .w_h(text_width + padding * 2.0, text_height + padding * 2.0)
        .color(srgba(40u8, 44u8, 50u8, 230u8));

    // Border
    draw.rect()
        .x_y(tooltip_x, tooltip_y)
        .w_h(text_width + padding * 2.0, text_height + padding * 2.0)
        .no_fill()
        .stroke(srgba(100u8, 100u8, 100u8, 150u8))
        .stroke_weight(1.0);

    // Time text
    draw.text(time_str)
        .x_y(tooltip_x, tooltip_y)
        .color(colors::TEXT_PRIMARY)
        .font_size(12)
        .w(text_width);

    // Vertical line from cursor to terrain
    let position = layout.x_to_position(mouse_x);
    let x = layout.position_to_x(position);

    draw.line()
        .start(pt2(x, mouse_y))
        .end(pt2(x, layout.bottom))
        .color(srgba(200u8, 200u8, 200u8, 60u8))
        .weight(1.0);
}

