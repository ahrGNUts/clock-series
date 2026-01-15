//! Geometry module - Mathematical layer computations for the Temporal Grammar clock
//!
//! Provides pure functions for computing the three visual layers (hour polygon,
//! minute superellipse, second phase ring) and applying timezone/DST transforms.

use nannou::prelude::*;
use shared::DstChange;
use std::f32::consts::{PI, TAU};

/// Data for the phase ring (second layer)
#[derive(Debug, Clone)]
pub struct PhaseRing {
    /// Center of the ring
    pub center: Point2,
    /// Radius of the ring
    pub radius: f32,
    /// Positions of all 60 marks
    pub marks: Vec<Point2>,
    /// Index of the highlighted mark (0-59)
    pub highlighted_index: usize,
    /// Angle of the needle in radians
    pub needle_angle: f32,
}

/// Data for a DST knot visualization
#[derive(Debug, Clone)]
pub struct DstKnot {
    /// Anchor position on the minute layer boundary
    pub anchor: Point2,
    /// Control points for the bezier loop (2 lobes)
    pub control_points: [Point2; 4],
    /// Amplitude of the knot (0.0 to max)
    pub amplitude: f32,
    /// Whether this is an upcoming or just-occurred transition
    pub is_upcoming: bool,
}

/// Computed geometry parameters for decode mode display
#[derive(Debug, Clone)]
pub struct GeometryParams {
    /// Hour value (1-12)
    pub hour: u32,
    /// Vertex count of hour polygon
    pub vertex_count: u32,
    /// Minute value (0-59)
    pub minute: u32,
    /// Superellipse exponent
    pub exponent: f32,
    /// Minute layer rotation in degrees
    pub minute_rotation_deg: f32,
    /// Second value (0-59)
    pub second: u32,
    /// Phase angle in degrees
    pub phase_deg: f32,
    /// Timezone rotation in degrees
    pub tz_rotation_deg: f32,
    /// Timezone skew X factor
    pub tz_skew_x: f32,
    /// Whether DST is active
    pub is_dst: bool,
}

/// Compute the hour polygon (foundation layer)
///
/// Creates a regular polygon with `3 + hour12` vertices.
/// - `h`: hour in 12-hour format (1-12)
/// - `min_dim`: minimum of canvas width/height
/// - `center`: center point of the polygon
pub fn compute_hour_polygon(h: u32, min_dim: f32, center: Point2) -> Vec<Point2> {
    let vertex_count = 3 + h;
    let r1 = 0.28 * min_dim;
    
    // Rotation based on hour: (h / 12) * 30 degrees
    let rot_hour = (h as f32 / 12.0) * 30.0_f32.to_radians();
    
    let mut points = Vec::with_capacity(vertex_count as usize);
    
    for k in 0..vertex_count {
        // Start at -90 degrees (12 o'clock position) + hour rotation
        let theta = -PI / 2.0 + rot_hour + (k as f32) * (TAU / vertex_count as f32);
        let x = center.x + r1 * theta.cos();
        let y = center.y + r1 * theta.sin();
        points.push(pt2(x, y));
    }
    
    points
}

/// Compute the minute superellipse (tension skin layer)
///
/// Creates a superellipse whose exponent varies with minute value.
/// - `m`: minute (0-59)
/// - `min_dim`: minimum of canvas width/height
/// - `center`: center point
/// - `samples`: number of points to sample (default 256)
pub fn compute_superellipse(m: u32, min_dim: f32, center: Point2, samples: usize) -> Vec<Point2> {
    let r2x = 0.40 * min_dim;
    let r2y = 0.22 * min_dim;
    
    // Exponent varies from 1.2 (m=0) to 4.0 (m=59)
    let e = 1.2 + 2.8 * (m as f32 / 59.0);
    
    // Start with the shape pointing at 12 o'clock (-30°), then rotate 360° clockwise over the hour
    let rot_min = -PI / 6.0 - (m as f32 / 60.0) * TAU;
    
    let mut points = Vec::with_capacity(samples);
    
    for i in 0..samples {
        let t = (i as f32 / samples as f32) * TAU;
        
        // Superellipse parametric formula
        let cos_t = t.cos();
        let sin_t = t.sin();
        
        let x = r2x * cos_t.signum() * cos_t.abs().powf(2.0 / e);
        let y = r2y * sin_t.signum() * sin_t.abs().powf(2.0 / e);
        
        // Apply rotation around center
        let rotated_x = x * rot_min.cos() - y * rot_min.sin();
        let rotated_y = x * rot_min.sin() + y * rot_min.cos();
        
        points.push(pt2(center.x + rotated_x, center.y + rotated_y));
    }
    
    points
}

/// Get the superellipse exponent for a given minute value
pub fn get_superellipse_exponent(m: u32) -> f32 {
    1.2 + 2.8 * (m as f32 / 59.0)
}

/// Get the minute rotation in degrees (starts at -30°, rotates 360° clockwise)
pub fn get_minute_rotation_deg(m: u32) -> f32 {
    -30.0 - (m as f32 / 60.0) * 360.0
}

/// Compute the second phase ring
///
/// Creates 60 marks around a ring with the current second highlighted.
/// - `s`: second (0-59)
/// - `second_fraction`: fractional part of second for smooth animation
/// - `min_dim`: minimum of canvas width/height
/// - `center`: center point
/// - `reduced_motion`: if true, needle snaps to discrete positions
pub fn compute_phase_ring(
    s: u32,
    second_fraction: f64,
    min_dim: f32,
    center: Point2,
    reduced_motion: bool,
) -> PhaseRing {
    let r3 = 0.46 * min_dim;
    
    let mut marks = Vec::with_capacity(60);
    
    for i in 0..60 {
        // Start at 12 o'clock (-90 degrees) and go clockwise
        let angle = -PI / 2.0 - (i as f32 / 60.0) * TAU;
        let x = center.x + r3 * angle.cos();
        let y = center.y + r3 * angle.sin();
        marks.push(pt2(x, y));
    }
    
    // Needle angle - smooth or snapping based on reduced motion
    let needle_angle = if reduced_motion {
        -PI / 2.0 - (s as f32 / 60.0) * TAU
    } else {
        let smooth_s = s as f64 + second_fraction;
        -PI / 2.0 - (smooth_s as f32 / 60.0) * TAU
    };
    
    PhaseRing {
        center,
        radius: r3,
        marks,
        highlighted_index: s as usize,
        needle_angle,
    }
}

/// Apply timezone reframing transform to a set of points
///
/// - `points`: input points
/// - `offset_minutes`: UTC offset in minutes (e.g., -480 for UTC-8)
/// - `is_dst`: whether DST is currently active
/// - `center`: center point for rotation
pub fn apply_tz_transform(
    points: &[Point2],
    offset_minutes: i32,
    is_dst: bool,
    center: Point2,
) -> Vec<Point2> {
    // tzRot = (offset / 60) * 7.5 degrees
    let tz_rot = (offset_minutes as f32 / 60.0) * 7.5_f32.to_radians();
    
    // tzSkewX = clamp((offset % 60) / 60, -1..1) * 0.10
    let remainder = (offset_minutes % 60) as f32 / 60.0;
    let tz_skew_x = remainder.clamp(-1.0, 1.0) * 0.10;
    
    // Additional DST transform
    let dst_extra_rot = if is_dst { 5.0_f32.to_radians() } else { 0.0 };
    let total_rot = tz_rot + dst_extra_rot;
    
    points
        .iter()
        .map(|p| {
            // Translate to origin
            let x = p.x - center.x;
            let y = p.y - center.y;
            
            // Apply skew
            let skewed_x = x + y * tz_skew_x;
            let skewed_y = y;
            
            // Apply rotation
            let rotated_x = skewed_x * total_rot.cos() - skewed_y * total_rot.sin();
            let rotated_y = skewed_x * total_rot.sin() + skewed_y * total_rot.cos();
            
            // Translate back
            pt2(center.x + rotated_x, center.y + rotated_y)
        })
        .collect()
}

/// Apply timezone transform specifically to the minute layer (with DST shear)
pub fn apply_tz_transform_minute_layer(
    points: &[Point2],
    offset_minutes: i32,
    is_dst: bool,
    center: Point2,
) -> Vec<Point2> {
    let tz_rot = (offset_minutes as f32 / 60.0) * 7.5_f32.to_radians();
    let remainder = (offset_minutes % 60) as f32 / 60.0;
    let tz_skew_x = remainder.clamp(-1.0, 1.0) * 0.10;
    
    let dst_extra_rot = if is_dst { 5.0_f32.to_radians() } else { 0.0 };
    let dst_shear_y = if is_dst { 0.06 } else { 0.0 };
    let total_rot = tz_rot + dst_extra_rot;
    
    points
        .iter()
        .map(|p| {
            let x = p.x - center.x;
            let y = p.y - center.y;
            
            // Apply skew X
            let skewed_x = x + y * tz_skew_x;
            // Apply DST shear Y (only for minute layer)
            let skewed_y = y + x * dst_shear_y;
            
            // Apply rotation
            let rotated_x = skewed_x * total_rot.cos() - skewed_y * total_rot.sin();
            let rotated_y = skewed_x * total_rot.sin() + skewed_y * total_rot.cos();
            
            pt2(center.x + rotated_x, center.y + rotated_y)
        })
        .collect()
}

/// Get timezone rotation in degrees
pub fn get_tz_rotation_deg(offset_minutes: i32, is_dst: bool) -> f32 {
    let base = (offset_minutes as f32 / 60.0) * 7.5;
    if is_dst { base + 5.0 } else { base }
}

/// Get timezone skew X factor
pub fn get_tz_skew_x(offset_minutes: i32) -> f32 {
    let remainder = (offset_minutes % 60) as f32 / 60.0;
    remainder.clamp(-1.0, 1.0) * 0.10
}

/// Compute DST knot if a transition is upcoming or just occurred
///
/// Returns None if no DST change is within the detection window.
pub fn compute_dst_knot(
    dst_change: &DstChange,
    offset_minutes: i32,
    is_dst: bool,
    min_dim: f32,
    center: Point2,
    now_utc: chrono::DateTime<chrono::Utc>,
) -> Option<DstKnot> {
    let tz_rot = get_tz_rotation_deg(offset_minutes, is_dst).to_radians();
    
    match dst_change {
        DstChange::Upcoming { instant, .. } => {
            // Calculate remaining seconds
            let remaining = (*instant - now_utc).num_seconds() as f32;
            let total_window = 24.0 * 3600.0; // 24 hours in seconds
            
            // u = clamp(1 - remaining / (24*3600), 0..1)
            let u = (1.0 - remaining / total_window).clamp(0.0, 1.0);
            
            // A = u * (0.08 * minDim)
            let amplitude = u * (0.08 * min_dim);
            
            if amplitude < 1.0 {
                return None; // Too small to render
            }
            
            Some(compute_knot_geometry(
                amplitude,
                tz_rot,
                min_dim,
                center,
                true,
            ))
        }
        DstChange::JustOccurred { instant, .. } => {
            // Decay over 2 hours
            let elapsed = (now_utc - *instant).num_seconds() as f32;
            let decay_window = 2.0 * 3600.0; // 2 hours
            
            if elapsed > decay_window {
                return None;
            }
            
            let decay_factor = 1.0 - elapsed / decay_window;
            let max_amplitude = 0.08 * min_dim;
            let amplitude = decay_factor * max_amplitude;
            
            if amplitude < 1.0 {
                return None;
            }
            
            Some(compute_knot_geometry(
                amplitude,
                tz_rot,
                min_dim,
                center,
                false,
            ))
        }
        DstChange::None => None,
    }
}

/// Helper to compute the knot geometry given amplitude
fn compute_knot_geometry(
    amplitude: f32,
    tz_rot: f32,
    min_dim: f32,
    center: Point2,
    is_upcoming: bool,
) -> DstKnot {
    // Anchor angle: 45 degrees + tzRot
    let anchor_angle = 45.0_f32.to_radians() + tz_rot;
    
    // Position on the minute layer boundary (use R2x as approximate radius)
    let r2x = 0.40 * min_dim;
    let anchor = pt2(
        center.x + r2x * anchor_angle.cos(),
        center.y + r2x * anchor_angle.sin(),
    );
    
    // Create a 2-lobed bezier loop
    // Control points extend outward from anchor
    let outward_dir = vec2(anchor_angle.cos(), anchor_angle.sin());
    let tangent_dir = vec2(-anchor_angle.sin(), anchor_angle.cos());
    
    let control_points = [
        anchor + outward_dir * amplitude + tangent_dir * amplitude * 0.5,
        anchor + outward_dir * amplitude * 1.5,
        anchor + outward_dir * amplitude - tangent_dir * amplitude * 0.5,
        anchor + outward_dir * amplitude * 0.5,
    ];
    
    DstKnot {
        anchor,
        control_points,
        amplitude,
        is_upcoming,
    }
}

/// Generate a textual description of the current geometry state
pub fn generate_diagram_description(
    params: &GeometryParams,
    tz_name: &str,
) -> String {
    let dst_status = if params.is_dst { "active" } else { "inactive" };
    
    format!(
        "Hour: {} ({}-sided polygon). \
         Minute: {} (superellipse e={:.1}, {:.0}° rotation). \
         Second: {} (mark {} of 60 highlighted). \
         Timezone: {} rotated {:.1}°, skew {:.2}. \
         DST {}.",
        params.hour,
        params.vertex_count,
        params.minute,
        params.exponent,
        params.minute_rotation_deg,
        params.second,
        params.second,
        tz_name,
        params.tz_rotation_deg,
        params.tz_skew_x,
        dst_status
    )
}

/// Compute all geometry parameters for the current time
pub fn compute_geometry_params(
    hour12: u32,
    minute: u32,
    second: u32,
    offset_minutes: i32,
    is_dst: bool,
) -> GeometryParams {
    GeometryParams {
        hour: hour12,
        vertex_count: 3 + hour12,
        minute,
        exponent: get_superellipse_exponent(minute),
        minute_rotation_deg: get_minute_rotation_deg(minute),
        second,
        phase_deg: (second as f32 / 60.0) * 360.0,
        tz_rotation_deg: get_tz_rotation_deg(offset_minutes, is_dst),
        tz_skew_x: get_tz_skew_x(offset_minutes),
        is_dst,
    }
}

/// Apply view transform (pan and zoom) to a point
pub fn apply_view_transform(point: Point2, offset: Vec2, zoom: f32, center: Point2) -> Point2 {
    let relative = point - center;
    center + (relative * zoom) + offset
}

/// Apply view transform to a set of points
pub fn apply_view_transform_points(
    points: &[Point2],
    offset: Vec2,
    zoom: f32,
    center: Point2,
) -> Vec<Point2> {
    points
        .iter()
        .map(|p| apply_view_transform(*p, offset, zoom, center))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hour_polygon_vertex_count() {
        for h in 1..=12 {
            let polygon = compute_hour_polygon(h, 100.0, pt2(0.0, 0.0));
            assert_eq!(polygon.len(), (3 + h) as usize);
        }
    }

    #[test]
    fn test_superellipse_exponent_range() {
        let e_min = get_superellipse_exponent(0);
        let e_max = get_superellipse_exponent(59);
        assert!((e_min - 1.2).abs() < 0.01);
        assert!((e_max - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_phase_ring_marks() {
        let ring = compute_phase_ring(30, 0.0, 100.0, pt2(0.0, 0.0), false);
        assert_eq!(ring.marks.len(), 60);
        assert_eq!(ring.highlighted_index, 30);
    }
}

