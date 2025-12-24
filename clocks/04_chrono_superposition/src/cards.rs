//! Card module - Zone card data model, ordering logic, and geometry calculations
//!
//! Handles the deterministic ordering of zone cards and their visual geometry
//! including stacking offsets, rotation, and parallax effects.

use chrono::Offset;
use chrono_tz::Tz;
use nannou::prelude::*;

use crate::drawing::CoreLayout;

/// Card dimensions
pub const CARD_WIDTH: f32 = 280.0;
pub const CARD_HEIGHT: f32 = 160.0;

/// Stacking offsets per card
const STACK_OFFSET_X: f32 = 18.0;
const STACK_OFFSET_Y: f32 = -12.0;

/// Rotation per card (degrees, alternating sign)
const ROTATION_DEG: f32 = 0.6;

/// Parallax strength
const PARALLAX_BASE: f32 = 6.0;
const PARALLAX_DEPTH_FACTOR: f32 = 0.15;

/// Geometry for a single card in the deck
#[derive(Debug, Clone)]
pub struct CardGeometry {
    /// Card index in display order (0 = dominant/top)
    #[allow(dead_code)]
    pub index: usize,
    /// Translation offset from center
    pub offset: Point2,
    /// Rotation angle in radians
    pub rotation: f32,
    /// Scale factor (for collapse animation)
    pub scale: f32,
    /// Opacity (0.0 - 1.0)
    pub opacity: f32,
}

impl CardGeometry {
    /// Compute geometry for a card at the given index
    pub fn compute(
        index: usize,
        _total_cards: usize,
        focus_strength: f32,
        pointer_delta: Option<Point2>,
        reduced_motion: bool,
    ) -> Self {
        let i = index as f32;

        // Base stacking offset (cards stack down and to the right)
        let base_offset_x = i * STACK_OFFSET_X;
        let base_offset_y = i * STACK_OFFSET_Y;

        // Parallax effect based on pointer position
        let parallax_offset = if reduced_motion {
            pt2(0.0, 0.0)
        } else if let Some(delta) = pointer_delta {
            let depth_factor = 1.0 + i * PARALLAX_DEPTH_FACTOR;
            pt2(
                delta.x * depth_factor * PARALLAX_BASE,
                delta.y * depth_factor * PARALLAX_BASE,
            )
        } else {
            pt2(0.0, 0.0)
        };

        // Combine offsets, reducing spread as focus_strength increases
        let spread_factor = 1.0 - focus_strength;
        let offset = pt2(
            (base_offset_x + parallax_offset.x) * spread_factor,
            (base_offset_y + parallax_offset.y) * spread_factor,
        );

        // Rotation (alternating sign, disabled in reduced motion)
        let rotation = if reduced_motion {
            0.0
        } else {
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            (i * ROTATION_DEG * sign * spread_factor).to_radians()
        };

        // Scale - slightly smaller for cards further back
        let scale = 1.0 - (i * 0.02 * spread_factor).min(0.15);

        // Opacity - cards further back are slightly more transparent
        let opacity = 1.0 - (i * 0.08 * spread_factor).min(0.4);

        Self {
            index,
            offset,
            rotation,
            scale,
            opacity,
        }
    }

    /// Get the bounding rectangle for this card given the layout
    pub fn card_rect(&self, layout: &CoreLayout) -> Rect {
        let center_x = layout.center_x + self.offset.x;
        let center_y = layout.center_y + self.offset.y;
        let half_w = CARD_WIDTH * self.scale / 2.0;
        let half_h = CARD_HEIGHT * self.scale / 2.0;

        Rect::from_x_y_w_h(center_x, center_y, half_w * 2.0, half_h * 2.0)
    }
}

/// Compute the deterministic display order for zones
///
/// Order:
/// 1. Dominant zone first
/// 2. Favorites next (in stable order)
/// 3. Remaining zones by UTC offset ascending, then lexicographic name
pub fn compute_display_order(
    selected_zones: &[Tz],
    dominant_zone: Tz,
    favorites: &[Tz],
) -> Vec<Tz> {
    let mut result = Vec::with_capacity(selected_zones.len());

    // 1. Dominant zone first
    result.push(dominant_zone);

    // 2. Favorites next (maintaining their order in the favorites list)
    for &fav in favorites {
        if fav != dominant_zone && selected_zones.contains(&fav) && !result.contains(&fav) {
            result.push(fav);
        }
    }

    // 3. Remaining zones sorted by UTC offset, then name
    let mut remaining: Vec<Tz> = selected_zones
        .iter()
        .filter(|&&z| !result.contains(&z))
        .copied()
        .collect();

    remaining.sort_by(|a, b| {
        // Get current UTC offsets
        let now = chrono::Utc::now();
        let offset_a = now.with_timezone(a).offset().fix().local_minus_utc();
        let offset_b = now.with_timezone(b).offset().fix().local_minus_utc();

        // Sort by offset first, then by name
        offset_a
            .cmp(&offset_b)
            .then_with(|| a.name().cmp(b.name()))
    });

    result.extend(remaining);
    result
}

/// Data for comparing a zone to the dominant zone
#[derive(Debug, Clone)]
pub struct ZoneComparison {
    /// Hours difference from dominant zone
    pub delta_hours: i32,
    /// Days difference from dominant zone (-1, 0, +1)
    pub delta_days: i32,
    /// Whether DST status differs from dominant zone
    pub dst_differs: bool,
}

impl ZoneComparison {
    /// Compute comparison data between a zone and the dominant zone
    pub fn compute(
        zone_offset_minutes: i32,
        zone_is_dst: bool,
        zone_day_index: i32,
        dominant_offset_minutes: i32,
        dominant_is_dst: bool,
        dominant_day_index: i32,
    ) -> Self {
        let delta_minutes = zone_offset_minutes - dominant_offset_minutes;
        let delta_hours = (delta_minutes as f32 / 60.0).round() as i32;
        let delta_days = zone_day_index - dominant_day_index;
        let dst_differs = zone_is_dst != dominant_is_dst;

        Self {
            delta_hours,
            delta_days,
            dst_differs,
        }
    }

    /// Format as a delta string like "+3h" or "−8h"
    pub fn format_hours(&self) -> String {
        if self.delta_hours == 0 {
            String::new()
        } else if self.delta_hours > 0 {
            format!("+{}h", self.delta_hours)
        } else {
            format!("{}h", self.delta_hours)
        }
    }

    /// Format day delta as "Yesterday", "Today", "Tomorrow"
    pub fn format_day(&self) -> Option<&'static str> {
        match self.delta_days {
            -1 => Some("Yesterday"),
            0 => None,
            1 => Some("Tomorrow"),
            _ => Some("Different day"),
        }
    }
}

/// Compute the "wall minutes" for a zone's local time
///
/// Used for composite readout calculations:
/// wallMinutes = localDayIndex*1440 + localHour24*60 + localMinute
#[allow(dead_code)]
pub fn compute_wall_minutes(hour24: u32, minute: u32, day_index: i32) -> i32 {
    day_index * 1440 + (hour24 as i32) * 60 + (minute as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_order() {
        let zones: Vec<Tz> = vec![
            "America/New_York".parse().unwrap(),
            "America/Los_Angeles".parse().unwrap(),
            "Europe/London".parse().unwrap(),
            "Asia/Tokyo".parse().unwrap(),
        ];
        let dominant: Tz = "Europe/London".parse().unwrap();
        let favorites: Vec<Tz> = vec!["Asia/Tokyo".parse().unwrap()];

        let order = compute_display_order(&zones, dominant, &favorites);

        // Dominant should be first
        assert_eq!(order[0], dominant);
        // Favorite should be second (since it's not dominant)
        assert_eq!(order[1], "Asia/Tokyo".parse::<Tz>().unwrap());
    }

    #[test]
    fn test_zone_comparison() {
        let comp = ZoneComparison::compute(
            -480, // UTC-8 (LA)
            false,
            0,    // today
            0,    // UTC
            false,
            0, // today
        );

        assert_eq!(comp.delta_hours, -8);
        assert_eq!(comp.delta_days, 0);
        assert!(!comp.dst_differs);
    }
}

