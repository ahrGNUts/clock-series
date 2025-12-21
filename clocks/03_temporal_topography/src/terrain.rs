//! Terrain module - day domain math, terrain elevation function, and DST fault logic
//!
//! Handles the mapping between time and topographic terrain coordinates,
//! including special handling for DST transitions that create gaps or overlaps.

use chrono::{DateTime, Datelike, Duration, NaiveTime, Offset, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use std::f32::consts::TAU;

/// Information about a DST fault line within the current day
#[derive(Debug, Clone)]
pub struct DstFault {
    /// Normalized position in the day [0..1] where the fault occurs
    pub position: f32,
    /// Width of the fault band in normalized units
    pub width: f32,
    /// Delta in minutes (positive = spring forward gap, negative = fall back overlap)
    pub delta_minutes: i32,
    /// Label for the first occurrence (before transition for fall back)
    pub label_a: Option<String>,
    /// Label for the second occurrence (after transition for fall back)
    pub label_b: Option<String>,
}

/// Day domain information for terrain rendering
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DayDomain {
    /// Seconds since local midnight for the current instant
    pub seconds_since_midnight: i64,
    /// Total length of the day in seconds (usually 86400, but can be 82800 or 90000 for DST)
    pub day_length_seconds: i64,
    /// Normalized position [0..1] of the current instant within the day
    pub normalized_position: f32,
    /// Any DST faults occurring within this day
    pub dst_faults: Vec<DstFault>,
    /// Local midnight instant (UTC)
    pub midnight_utc: DateTime<Utc>,
    /// Next local midnight instant (UTC)
    pub next_midnight_utc: DateTime<Utc>,
}

impl DayDomain {
    /// Compute the day domain for a given instant and timezone
    pub fn compute(instant: DateTime<Utc>, tz: Tz) -> Self {
        let local = instant.with_timezone(&tz);
        
        // Get local midnight (start of today)
        let local_date = local.date_naive();
        let midnight_local = tz
            .from_local_datetime(&local_date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()))
            .single()
            .unwrap_or_else(|| {
                // Handle ambiguous/nonexistent midnight (rare DST edge case)
                tz.from_local_datetime(&local_date.and_time(NaiveTime::from_hms_opt(1, 0, 0).unwrap()))
                    .single()
                    .unwrap()
            });
        
        // Get next midnight
        let next_date = local_date + chrono::Duration::days(1);
        let next_midnight_local = tz
            .from_local_datetime(&next_date.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()))
            .single()
            .unwrap_or_else(|| {
                tz.from_local_datetime(&next_date.and_time(NaiveTime::from_hms_opt(1, 0, 0).unwrap()))
                    .single()
                    .unwrap()
            });
        
        let midnight_utc = midnight_local.with_timezone(&Utc);
        let next_midnight_utc = next_midnight_local.with_timezone(&Utc);
        
        // Calculate day length in seconds
        let day_length_seconds = (next_midnight_utc - midnight_utc).num_seconds();
        
        // Calculate seconds since midnight
        let seconds_since_midnight = (instant - midnight_utc).num_seconds();
        
        // Normalize position
        let normalized_position = if day_length_seconds > 0 {
            (seconds_since_midnight as f32 / day_length_seconds as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        
        // Detect DST faults within this day
        let dst_faults = Self::detect_dst_faults(tz, midnight_utc, next_midnight_utc, day_length_seconds);
        
        Self {
            seconds_since_midnight,
            day_length_seconds,
            normalized_position,
            dst_faults,
            midnight_utc,
            next_midnight_utc,
        }
    }
    
    /// Convert a normalized position [0..1] to seconds since midnight
    pub fn position_to_ssm(&self, p: f32) -> i64 {
        (p * self.day_length_seconds as f32) as i64
    }
    
    /// Convert seconds since midnight to normalized position [0..1]
    pub fn ssm_to_position(&self, ssm: i64) -> f32 {
        if self.day_length_seconds > 0 {
            (ssm as f32 / self.day_length_seconds as f32).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
    
    /// Snap a normalized position to the nearest minute boundary
    pub fn snap_to_minute(&self, p: f32) -> f32 {
        let ssm = self.position_to_ssm(p);
        let snapped_ssm = (ssm / 60) * 60;
        self.ssm_to_position(snapped_ssm)
    }
    
    /// Check if a position falls within a DST gap (nonexistent time)
    pub fn is_in_gap(&self, p: f32) -> bool {
        for fault in &self.dst_faults {
            if fault.delta_minutes > 0 {
                // Spring forward gap
                if p >= fault.position && p < fault.position + fault.width {
                    return true;
                }
            }
        }
        false
    }
    
    /// Check if a position falls within a DST overlap (ambiguous time)
    pub fn is_in_overlap(&self, p: f32) -> Option<&DstFault> {
        for fault in &self.dst_faults {
            if fault.delta_minutes < 0 {
                // Fall back overlap
                if p >= fault.position && p < fault.position + fault.width {
                    return Some(fault);
                }
            }
        }
        None
    }
    
    /// Detect DST faults within the day
    fn detect_dst_faults(
        tz: Tz,
        midnight_utc: DateTime<Utc>,
        next_midnight_utc: DateTime<Utc>,
        day_length_seconds: i64,
    ) -> Vec<DstFault> {
        let mut faults = Vec::new();
        
        // Sample hourly to detect offset changes
        let mut current = midnight_utc;
        let mut prev_offset = current.with_timezone(&tz).offset().fix().local_minus_utc();
        
        while current < next_midnight_utc {
            let next = current + Duration::hours(1);
            if next > next_midnight_utc {
                break;
            }
            
            let next_offset = next.with_timezone(&tz).offset().fix().local_minus_utc();
            
            if next_offset != prev_offset {
                // Found a transition
                let delta_seconds = next_offset - prev_offset;
                let delta_minutes = delta_seconds / 60;
                
                // Calculate position in normalized day coordinates
                let transition_ssm = (current - midnight_utc).num_seconds();
                let position = transition_ssm as f32 / day_length_seconds as f32;
                
                // Width of the fault band
                let width = (delta_minutes.abs() * 60) as f32 / day_length_seconds as f32;
                
                let (label_a, label_b) = if delta_minutes < 0 {
                    // Fall back - repeated hour
                    (Some("(A)".to_string()), Some("(B)".to_string()))
                } else {
                    // Spring forward - gap
                    (None, None)
                };
                
                faults.push(DstFault {
                    position,
                    width,
                    delta_minutes,
                    label_a,
                    label_b,
                });
            }
            
            prev_offset = next_offset;
            current = next;
        }
        
        faults
    }
}

/// Terrain parameters extracted from time data
#[derive(Debug, Clone, Copy)]
pub struct TerrainParams {
    /// Hour in 12-hour format (1-12)
    pub hour12: u32,
    /// Minute (0-59)
    pub minute: u32,
    /// Second (0-59)
    pub second: u32,
    /// Day of year (1-366)
    pub day_of_year: u32,
}

impl TerrainParams {
    /// Create terrain params from a DateTime
    pub fn from_datetime(dt: DateTime<Tz>) -> Self {
        let hour24 = dt.hour();
        let hour12 = match hour24 {
            0 => 12,
            1..=12 => hour24,
            _ => hour24 - 12,
        };
        
        Self {
            hour12,
            minute: dt.minute(),
            second: dt.second(),
            day_of_year: dt.ordinal(),
        }
    }
}

/// Compute the terrain elevation at a normalized position p in [0..1]
///
/// The elevation is computed using a deterministic function based on:
/// - The position p (horizontal location on the day map)
/// - The current hour, minute, second (creates the terrain shape)
/// - The day of year (adds daily variation)
///
/// Returns a value in [-1..1]
pub fn terrain_elevation(p: f32, params: &TerrainParams) -> f32 {
    let h_norm = params.hour12 as f32 / 12.0;
    let m_norm = params.minute as f32 / 60.0;
    let s_norm = params.second as f32 / 60.0;
    let d_norm = params.day_of_year as f32 / 366.0;
    
    let elevation = 0.50 * (TAU * (p + h_norm)).sin()
        + 0.25 * (TAU * (4.0 * p + m_norm)).sin()
        + 0.15 * (TAU * (16.0 * p + s_norm)).sin()
        + 0.10 * (TAU * (p + d_norm)).sin();
    
    elevation.clamp(-1.0, 1.0)
}

/// Generate terrain samples for rendering
///
/// Returns a vector of (x_normalized, elevation) pairs
#[allow(dead_code)]
pub fn generate_terrain_samples(
    params: &TerrainParams,
    day_domain: &DayDomain,
    sample_count: usize,
) -> Vec<(f32, f32, bool)> {
    let mut samples = Vec::with_capacity(sample_count);
    
    for i in 0..sample_count {
        let p = i as f32 / (sample_count - 1) as f32;
        
        // Check if this position is in a DST gap
        let in_gap = day_domain.is_in_gap(p);
        
        let elevation = if in_gap {
            // Don't compute elevation for gap regions
            0.0
        } else {
            terrain_elevation(p, params)
        };
        
        samples.push((p, elevation, in_gap));
    }
    
    samples
}

/// Hour boundary information for grid rendering
#[derive(Debug, Clone)]
pub struct HourBoundary {
    /// Normalized position [0..1]
    pub position: f32,
    /// Hour label (e.g., "12 AM", "3 PM")
    pub label: String,
    /// Whether this is midnight
    pub is_midnight: bool,
    /// Whether this is the next day's midnight (shows "(next)" below)
    pub is_next_day: bool,
    /// Suffix for DST ambiguous hours (A/B)
    pub suffix: Option<String>,
}

/// Generate hour boundaries for grid rendering
pub fn generate_hour_boundaries(_tz: Tz, day_domain: &DayDomain) -> Vec<HourBoundary> {
    let mut boundaries = Vec::new();
    
    // Iterate through each hour of the day
    for hour in 0..=24 {
        let ssm = hour * 3600;
        
        if ssm > day_domain.day_length_seconds {
            break;
        }
        
        let position = day_domain.ssm_to_position(ssm);
        
        // Format hour label
        let hour_mod = hour % 24;
        let hour12 = match hour_mod {
            0 => 12,
            1..=12 => hour_mod,
            _ => hour_mod - 12,
        };
        let meridiem = if hour_mod < 12 { "AM" } else { "PM" };
        let is_midnight = hour_mod == 0;
        
        let label = if is_midnight {
            "12 AM".to_string()
        } else {
            format!("{} {}", hour12, meridiem)
        };
        
        // Mark if this is the next day's midnight
        let is_next_day = is_midnight && hour > 0;
        
        // Check for DST overlap at this hour
        let suffix = day_domain.is_in_overlap(position).map(|fault| {
            if position < fault.position + fault.width / 2.0 {
                fault.label_a.clone().unwrap_or_default()
            } else {
                fault.label_b.clone().unwrap_or_default()
            }
        });
        
        boundaries.push(HourBoundary {
            position,
            label,
            is_midnight,
            is_next_day,
            suffix,
        });
    }
    
    boundaries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_elevation_bounds() {
        let params = TerrainParams {
            hour12: 6,
            minute: 30,
            second: 45,
            day_of_year: 180,
        };
        
        for i in 0..100 {
            let p = i as f32 / 99.0;
            let e = terrain_elevation(p, &params);
            assert!(e >= -1.0 && e <= 1.0, "Elevation {} out of bounds at p={}", e, p);
        }
    }

    #[test]
    fn test_day_domain_normal_day() {
        let tz: Tz = "UTC".parse().unwrap();
        let now = Utc::now();
        let domain = DayDomain::compute(now, tz);
        
        // UTC should always have 86400 second days
        assert_eq!(domain.day_length_seconds, 86400);
        assert!(domain.dst_faults.is_empty());
    }
}

