//! Time Engine - Core time computation logic per the Global Time Engine Contract
//!
//! Provides timezone-aware time data, DST detection, and transition warnings.

use chrono::{DateTime, Datelike, Duration, Local, Offset, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// AM/PM indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Meridiem {
    AM,
    PM,
}

impl std::fmt::Display for Meridiem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Meridiem::AM => write!(f, "AM"),
            Meridiem::PM => write!(f, "PM"),
        }
    }
}

/// DST transition information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DstChange {
    /// No DST change within the detection window
    None,
    /// DST change upcoming within 24 hours
    Upcoming {
        /// When the change occurs (UTC)
        instant: DateTime<Utc>,
        /// Offset change in minutes (positive = spring forward, negative = fall back)
        delta_minutes: i32,
    },
    /// DST change just occurred within the last 24 hours
    JustOccurred {
        /// When the change occurred (UTC)
        instant: DateTime<Utc>,
        /// Offset change in minutes
        delta_minutes: i32,
    },
}

/// Validity status of time zone data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Validity {
    /// Everything is working correctly
    Ok,
    /// Time zone data is missing
    TzMissing,
    /// Time zone data may be stale
    TzDataStale,
    /// Unknown validity state
    Unknown,
}

/// Complete time data for a single render tick
#[derive(Debug, Clone)]
pub struct TimeData {
    /// Year (e.g., 2025)
    pub year: i32,
    /// Month (1-12)
    pub month: u32,
    /// Day of month (1-31)
    pub day: u32,
    /// Day of week
    pub weekday: Weekday,
    /// Hour in 12-hour format (1-12)
    pub hour12: u32,
    /// Hour in 24-hour format (0-23)
    pub hour24: u32,
    /// Minute (0-59)
    pub minute: u32,
    /// Second (0-59)
    pub second: u32,
    /// Fractional seconds (0.0-1.0) for smooth animations
    pub second_fraction: f64,
    /// AM/PM indicator
    pub meridiem: Meridiem,
    /// UTC offset in minutes (e.g., -480 for UTC-8)
    pub utc_offset_minutes: i32,
    /// Whether DST is currently active
    pub is_dst: bool,
    /// DST transition information
    pub dst_change: DstChange,
    /// Time zone abbreviation (e.g., "PST", "PDT")
    pub tz_abbrev: String,
    /// Validity of the time zone data
    pub validity: Validity,
    /// The raw DateTime for additional formatting needs
    pub local_datetime: DateTime<Tz>,
}

impl TimeData {
    /// Format the time as "hh:mm:ss"
    pub fn format_time(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            self.hour12, self.minute, self.second
        )
    }

    /// Format the date as "Weekday, Month Day, Year"
    pub fn format_date(&self) -> String {
        let month_name = match self.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        };
        format!(
            "{}, {} {}, {}",
            self.weekday,
            month_name,
            self.day,
            self.year
        )
    }

    /// Format the UTC offset as "UTC±hh:mm"
    pub fn format_utc_offset(&self) -> String {
        let sign = if self.utc_offset_minutes >= 0 { "+" } else { "-" };
        let abs_minutes = self.utc_offset_minutes.abs();
        let hours = abs_minutes / 60;
        let mins = abs_minutes % 60;
        format!("UTC{}{:02}:{:02}", sign, hours, mins)
    }

    /// Get an accessible description of the time
    pub fn accessible_description(&self) -> String {
        format!(
            "It is {} {} and {} seconds {}, {} Time.",
            self.hour12,
            self.minute,
            self.second,
            self.meridiem,
            self.tz_abbrev
        )
    }
}

/// Compute the current time data for a given timezone
pub fn compute_time_data(tz: Tz) -> TimeData {
    let now_utc = Utc::now();
    compute_time_data_at(tz, now_utc)
}

/// Compute time data for a given timezone at a specific instant
pub fn compute_time_data_at(tz: Tz, now_utc: DateTime<Utc>) -> TimeData {
    let local = now_utc.with_timezone(&tz);
    
    // Calculate 12-hour format
    let hour24 = local.hour();
    let hour12 = match hour24 {
        0 => 12,
        1..=12 => hour24,
        _ => hour24 - 12,
    };
    let meridiem = if hour24 < 12 { Meridiem::AM } else { Meridiem::PM };
    
    // Calculate fractional seconds for smooth animations
    let nanos = local.nanosecond();
    let second_fraction = nanos as f64 / 1_000_000_000.0;
    
    // Get UTC offset in minutes
    let offset = local.offset().fix();
    let utc_offset_minutes = offset.local_minus_utc() / 60;
    
    // Detect DST status and transitions
    let (is_dst, dst_change) = detect_dst_status(tz, now_utc);
    
    // Get timezone abbreviation
    let tz_abbrev = local.format("%Z").to_string();
    
    TimeData {
        year: local.year(),
        month: local.month(),
        day: local.day(),
        weekday: local.weekday(),
        hour12,
        hour24,
        minute: local.minute(),
        second: local.second(),
        second_fraction,
        meridiem,
        utc_offset_minutes,
        is_dst,
        dst_change,
        tz_abbrev,
        validity: Validity::Ok,
        local_datetime: local,
    }
}

/// Detect DST status and upcoming/recent transitions
fn detect_dst_status(tz: Tz, now_utc: DateTime<Utc>) -> (bool, DstChange) {
    let local_now = now_utc.with_timezone(&tz);
    let current_offset = local_now.offset().fix().local_minus_utc();
    
    // Check for standard vs daylight offset by comparing to winter
    // This is a heuristic - compare current offset to offset in January
    let january = tz
        .with_ymd_and_hms(local_now.year(), 1, 15, 12, 0, 0)
        .single();
    let july = tz
        .with_ymd_and_hms(local_now.year(), 7, 15, 12, 0, 0)
        .single();
    
    let is_dst = match (january, july) {
        (Some(jan), Some(jul)) => {
            let jan_offset = jan.offset().fix().local_minus_utc();
            let jul_offset = jul.offset().fix().local_minus_utc();
            // If offsets differ, we have DST. Current is DST if it matches the larger offset
            if jan_offset != jul_offset {
                current_offset == jan_offset.max(jul_offset)
            } else {
                false
            }
        }
        _ => false,
    };
    
    // Check for transitions in the next 24 hours
    let future = now_utc + Duration::hours(24);
    let future_local = future.with_timezone(&tz);
    let future_offset = future_local.offset().fix().local_minus_utc();
    
    if future_offset != current_offset {
        // Find approximate transition time by binary search
        let transition = find_transition_time(tz, now_utc, future, current_offset);
        let delta_minutes = (future_offset - current_offset) / 60;
        return (is_dst, DstChange::Upcoming {
            instant: transition,
            delta_minutes,
        });
    }
    
    // Check for transitions in the past 24 hours
    let past = now_utc - Duration::hours(24);
    let past_local = past.with_timezone(&tz);
    let past_offset = past_local.offset().fix().local_minus_utc();
    
    if past_offset != current_offset {
        let transition = find_transition_time(tz, past, now_utc, past_offset);
        let delta_minutes = (current_offset - past_offset) / 60;
        return (is_dst, DstChange::JustOccurred {
            instant: transition,
            delta_minutes,
        });
    }
    
    (is_dst, DstChange::None)
}

/// Find the approximate transition time using binary search
fn find_transition_time(
    tz: Tz,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    start_offset: i32,
) -> DateTime<Utc> {
    let mut low = start;
    let mut high = end;
    
    // Binary search to find transition within ~1 minute accuracy
    while high - low > Duration::minutes(1) {
        let mid = low + (high - low) / 2;
        let mid_local = mid.with_timezone(&tz);
        let mid_offset = mid_local.offset().fix().local_minus_utc();
        
        if mid_offset == start_offset {
            low = mid;
        } else {
            high = mid;
        }
    }
    
    high
}

/// Get the system's local timezone as a chrono-tz Tz
pub fn system_timezone() -> Option<Tz> {
    // Try to get the system timezone from the Local type
    let local_now = Local::now();
    let tz_name = local_now.format("%Z").to_string();
    
    // Try to parse as IANA timezone - this often doesn't work directly
    // Fall back to checking common patterns
    tz_name.parse::<Tz>().ok()
}

/// Parse a timezone string into a Tz, with fallback
pub fn parse_timezone(tz_str: &str) -> Result<Tz, String> {
    tz_str
        .parse::<Tz>()
        .map_err(|_| format!("Invalid timezone: {}", tz_str))
}

/// Get a list of all available timezones
pub fn all_timezones() -> Vec<Tz> {
    chrono_tz::TZ_VARIANTS.to_vec()
}

/// Search timezones by name (case-insensitive partial match)
pub fn search_timezones(query: &str) -> Vec<Tz> {
    let query_lower = query.to_lowercase();
    chrono_tz::TZ_VARIANTS
        .iter()
        .filter(|tz| tz.name().to_lowercase().contains(&query_lower))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_time_data() {
        let tz: Tz = "America/New_York".parse().unwrap();
        let data = compute_time_data(tz);
        assert!(data.hour12 >= 1 && data.hour12 <= 12);
        assert!(data.minute < 60);
        assert!(data.second < 60);
    }

    #[test]
    fn test_format_utc_offset() {
        let tz: Tz = "America/Los_Angeles".parse().unwrap();
        let data = compute_time_data(tz);
        let offset = data.format_utc_offset();
        assert!(offset.starts_with("UTC"));
    }

    #[test]
    fn test_search_timezones() {
        let results = search_timezones("New_York");
        assert!(!results.is_empty());
        assert!(results.iter().any(|tz| tz.name() == "America/New_York"));
    }
}

