//! Ribbon module - coordinate system, tick generation, and warp math
//!
//! Handles the mapping between time instants and screen coordinates,
//! generates tick marks at appropriate intervals, and computes DST warp effects.

use chrono::{DateTime, Duration, Timelike, Utc};
use chrono_tz::Tz;
use shared::DstTransition;

/// Available zoom levels in seconds per pixel
pub const ZOOM_LEVELS: [f32; 5] = [5.0, 10.0, 30.0, 60.0, 120.0];

/// Default zoom level index (30 sec/px)
pub const DEFAULT_ZOOM_INDEX: usize = 2;

/// Warp effect half-width in seconds (30 minutes)
const WARP_HALF_WIDTH: f32 = 1800.0;

/// Tick type for rendering different visual weights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickType {
    /// Hour boundary - heaviest weight, labeled
    Hour,
    /// 5-minute boundary - medium weight
    FiveMinute,
    /// Minute boundary - light weight
    Minute,
    /// Second tick - only shown near cursor
    Second,
    /// Midnight boundary - special, shows date
    Midnight,
}

/// A tick mark to be rendered on the ribbon
#[derive(Debug, Clone)]
pub struct Tick {
    /// The instant this tick represents
    pub instant: DateTime<Utc>,
    /// X position in pixels (relative to viewport center)
    pub x_position: f32,
    /// Type of tick for styling
    pub tick_type: TickType,
    /// Label text (if any)
    pub label: Option<String>,
}

/// Viewport state for the ribbon
#[derive(Debug, Clone)]
pub struct RibbonViewport {
    /// The instant at the center of the viewport (under the Now Cursor)
    pub center_instant: DateTime<Utc>,
    /// Seconds per pixel (zoom level)
    pub seconds_per_pixel: f32,
    /// Viewport width in pixels
    pub viewport_width: f32,
    /// Selected timezone for formatting labels
    pub timezone: Tz,
}

impl RibbonViewport {
    /// Create a new viewport
    pub fn new(center_instant: DateTime<Utc>, seconds_per_pixel: f32, viewport_width: f32, timezone: Tz) -> Self {
        Self {
            center_instant,
            seconds_per_pixel,
            viewport_width,
            timezone,
        }
    }

    /// Get the time span visible in the viewport (in seconds)
    pub fn visible_span_seconds(&self) -> i64 {
        (self.viewport_width * self.seconds_per_pixel) as i64
    }

    /// Get the instant at the left edge of the viewport
    pub fn left_instant(&self) -> DateTime<Utc> {
        self.center_instant - Duration::seconds(self.visible_span_seconds() / 2)
    }

    /// Get the instant at the right edge of the viewport
    pub fn right_instant(&self) -> DateTime<Utc> {
        self.center_instant + Duration::seconds(self.visible_span_seconds() / 2)
    }

    /// Convert an instant to an x position (relative to viewport center)
    pub fn instant_to_x(&self, instant: DateTime<Utc>) -> f32 {
        let delta_seconds = (instant - self.center_instant).num_milliseconds() as f32 / 1000.0;
        delta_seconds / self.seconds_per_pixel
    }

    /// Convert an x position to an instant
    #[allow(dead_code)]
    pub fn x_to_instant(&self, x: f32) -> DateTime<Utc> {
        let delta_seconds = (x * self.seconds_per_pixel) as i64;
        self.center_instant + Duration::seconds(delta_seconds)
    }

    /// Apply warp effect to an x position based on nearby DST transitions
    pub fn apply_warp(&self, x: f32, instant: DateTime<Utc>, transitions: &[DstTransition]) -> f32 {
        let mut warped_x = x;

        for transition in transitions {
            let d = (instant - transition.instant_utc).num_seconds() as f32;

            if d.abs() <= WARP_HALF_WIDTH {
                // Within warp zone
                let u = (d + WARP_HALF_WIDTH) / (2.0 * WARP_HALF_WIDTH);
                let smoothstep = u * u * (3.0 - 2.0 * u);
                let jump_px = (transition.delta_minutes * 60) as f32 / self.seconds_per_pixel;
                let warp_offset = jump_px * (smoothstep - 0.5);
                warped_x += warp_offset;
            }
        }

        warped_x
    }

    /// Generate all tick marks visible in the viewport
    pub fn generate_ticks(&self) -> Vec<Tick> {
        let mut ticks = Vec::new();
        let left = self.left_instant();
        let right = self.right_instant();

        // Generate hour ticks
        self.generate_hour_ticks(&mut ticks, left, right);

        // Generate 5-minute ticks (if zoomed in enough)
        if self.seconds_per_pixel <= 60.0 {
            self.generate_five_minute_ticks(&mut ticks, left, right);
        }

        // Generate minute ticks (if zoomed in enough)
        if self.seconds_per_pixel <= 30.0 {
            self.generate_minute_ticks(&mut ticks, left, right);
        }

        // Generate second ticks (only near center, if very zoomed in)
        if self.seconds_per_pixel <= 10.0 {
            self.generate_second_ticks(&mut ticks);
        }

        ticks
    }

    fn generate_hour_ticks(&self, ticks: &mut Vec<Tick>, left: DateTime<Utc>, right: DateTime<Utc>) {
        let left_local = left.with_timezone(&self.timezone);
        let right_local = right.with_timezone(&self.timezone);

        // Determine label interval based on zoom level to prevent overlapping
        // At high sec/px (zoomed out), show labels less frequently
        let label_interval: u32 = if self.seconds_per_pixel >= 120.0 {
            6 // Label every 6 hours when very zoomed out
        } else if self.seconds_per_pixel >= 60.0 {
            3 // Label every 3 hours
        } else {
            1 // Label every hour when zoomed in
        };

        // Start from the hour at or after left edge
        let mut current_hour = left_local
            .with_minute(0)
            .and_then(|dt| dt.with_second(0))
            .and_then(|dt| dt.with_nanosecond(0))
            .unwrap();

        if current_hour < left_local {
            current_hour = current_hour + Duration::hours(1);
        }

        while current_hour <= right_local {
            let instant = current_hour.with_timezone(&Utc);
            let x = self.instant_to_x(instant);

            // Check if this is midnight
            let is_midnight = current_hour.hour() == 0;
            let tick_type = if is_midnight {
                TickType::Midnight
            } else {
                TickType::Hour
            };

            // Only generate label at the appropriate interval
            let should_label = is_midnight || (current_hour.hour() % label_interval == 0);

            let label = if should_label {
                if is_midnight {
                    // Date label for midnight
                    Some(current_hour.format("%a %b %d").to_string())
                } else {
                    // Time label for regular hours
                    let hour12 = match current_hour.hour() {
                        0 => 12,
                        h if h <= 12 => h,
                        h => h - 12,
                    };
                    let meridiem = if current_hour.hour() < 12 { "AM" } else { "PM" };
                    Some(format!("{}:00 {}", hour12, meridiem))
                }
            } else {
                None
            };

            ticks.push(Tick {
                instant,
                x_position: x,
                tick_type,
                label,
            });

            current_hour = current_hour + Duration::hours(1);
        }
    }

    fn generate_five_minute_ticks(&self, ticks: &mut Vec<Tick>, left: DateTime<Utc>, right: DateTime<Utc>) {
        let left_local = left.with_timezone(&self.timezone);
        let right_local = right.with_timezone(&self.timezone);

        // Start from a 5-minute boundary
        let minute = left_local.minute();
        let aligned_minute = (minute / 5) * 5;
        let mut current = left_local
            .with_minute(aligned_minute)
            .and_then(|dt| dt.with_second(0))
            .and_then(|dt| dt.with_nanosecond(0))
            .unwrap();

        if current < left_local {
            current = current + Duration::minutes(5);
        }

        while current <= right_local {
            // Skip if already an hour tick
            if current.minute() != 0 {
                let instant = current.with_timezone(&Utc);
                let x = self.instant_to_x(instant);

                ticks.push(Tick {
                    instant,
                    x_position: x,
                    tick_type: TickType::FiveMinute,
                    label: None,
                });
            }

            current = current + Duration::minutes(5);
        }
    }

    fn generate_minute_ticks(&self, ticks: &mut Vec<Tick>, left: DateTime<Utc>, right: DateTime<Utc>) {
        let left_local = left.with_timezone(&self.timezone);
        let right_local = right.with_timezone(&self.timezone);

        let mut current = left_local
            .with_second(0)
            .and_then(|dt| dt.with_nanosecond(0))
            .unwrap();

        if current < left_local {
            current = current + Duration::minutes(1);
        }

        while current <= right_local {
            // Skip if already a 5-minute or hour tick
            if current.minute() % 5 != 0 {
                let instant = current.with_timezone(&Utc);
                let x = self.instant_to_x(instant);

                ticks.push(Tick {
                    instant,
                    x_position: x,
                    tick_type: TickType::Minute,
                    label: None,
                });
            }

            current = current + Duration::minutes(1);
        }
    }

    fn generate_second_ticks(&self, ticks: &mut Vec<Tick>) {
        // Only generate seconds within ±90 seconds of center
        const SECOND_RANGE: i64 = 90;

        for offset in -SECOND_RANGE..=SECOND_RANGE {
            let instant = self.center_instant + Duration::seconds(offset);
            let local = instant.with_timezone(&self.timezone);

            // Skip if already a minute tick
            if local.second() != 0 {
                let x = self.instant_to_x(instant);

                ticks.push(Tick {
                    instant,
                    x_position: x,
                    tick_type: TickType::Second,
                    label: None,
                });
            }
        }
    }
}

/// Format an instant for display at the cursor
pub fn format_cursor_time(instant: DateTime<Utc>, tz: Tz) -> String {
    let local = instant.with_timezone(&tz);
    let hour24 = local.hour();
    let hour12 = match hour24 {
        0 => 12,
        h if h <= 12 => h,
        h => h - 12,
    };
    let meridiem = if hour24 < 12 { "AM" } else { "PM" };

    format!(
        "{}:{:02}:{:02} {}",
        hour12,
        local.minute(),
        local.second(),
        meridiem
    )
}

/// Format an instant with date for display
#[allow(dead_code)]
pub fn format_cursor_datetime(instant: DateTime<Utc>, tz: Tz) -> String {
    let local = instant.with_timezone(&tz);
    let hour24 = local.hour();
    let hour12 = match hour24 {
        0 => 12,
        h if h <= 12 => h,
        h => h - 12,
    };
    let meridiem = if hour24 < 12 { "AM" } else { "PM" };

    format!(
        "{} {}:{:02}:{:02} {}",
        local.format("%a %b %d, %Y"),
        hour12,
        local.minute(),
        local.second(),
        meridiem
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_coordinate_conversion() {
        let tz: Tz = "UTC".parse().unwrap();
        let center = Utc::now();
        let viewport = RibbonViewport::new(center, 30.0, 800.0, tz);

        // Center should be at x=0
        assert!((viewport.instant_to_x(center) - 0.0).abs() < 0.001);

        // 30 seconds in the future should be at x=1
        let future = center + Duration::seconds(30);
        assert!((viewport.instant_to_x(future) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_warp_smoothstep() {
        // Verify smoothstep at boundaries
        let u_start = 0.0;
        let u_end = 1.0;
        let u_mid = 0.5;

        let ss_start = u_start * u_start * (3.0 - 2.0 * u_start);
        let ss_end = u_end * u_end * (3.0 - 2.0 * u_end);
        let ss_mid = u_mid * u_mid * (3.0 - 2.0 * u_mid);

        assert!((ss_start - 0.0).abs() < 0.001);
        assert!((ss_end - 1.0).abs() < 0.001);
        assert!((ss_mid - 0.5).abs() < 0.001);
    }
}

