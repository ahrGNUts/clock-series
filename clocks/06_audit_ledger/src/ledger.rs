//! Ledger data structures for the Audit Ledger Clock
//!
//! Provides LedgerEntry, MinuteBlock, HourChapter, and LedgerState for managing
//! the rolling window of time entries with hierarchical grouping.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use shared::{compute_time_data_at, DstChange, TimeData};
use std::collections::{HashSet, VecDeque};

/// Time range filter options (in minutes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRangeFilter {
    Minutes5,
    Minutes10,
    Minutes30,
    Minutes60,
}

impl TimeRangeFilter {
    pub fn as_seconds(&self) -> usize {
        match self {
            TimeRangeFilter::Minutes5 => 5 * 60,
            TimeRangeFilter::Minutes10 => 10 * 60,
            TimeRangeFilter::Minutes30 => 30 * 60,
            TimeRangeFilter::Minutes60 => 60 * 60,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TimeRangeFilter::Minutes5 => "5 min",
            TimeRangeFilter::Minutes10 => "10 min",
            TimeRangeFilter::Minutes30 => "30 min",
            TimeRangeFilter::Minutes60 => "60 min",
        }
    }

    pub fn all() -> &'static [TimeRangeFilter] {
        &[
            TimeRangeFilter::Minutes5,
            TimeRangeFilter::Minutes10,
            TimeRangeFilter::Minutes30,
            TimeRangeFilter::Minutes60,
        ]
    }
}

impl Default for TimeRangeFilter {
    fn default() -> Self {
        TimeRangeFilter::Minutes10
    }
}

/// DST badge information for a ledger entry
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DstBadge {
    /// No DST-related badge
    None,
    /// DST is currently active
    Active,
    /// This is a gap marker (spring forward)
    GapMarker { from: String, to: String },
    /// First pass through an overlapped hour (fall back)
    OverlapPass1,
    /// Second pass through an overlapped hour (fall back)
    OverlapPass2,
}

/// A single ledger entry representing one second
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    /// The UTC instant this entry represents
    pub instant_utc: DateTime<Utc>,
    /// Formatted 12h time string (e.g., "12:34:56 PM")
    pub local_timestamp: String,
    /// Current minute (0-59) - the "block" this entry belongs to
    pub block_id: u32,
    /// Current hour (0-23) - the "chapter" this entry belongs to
    pub chapter_id: u32,
    /// UTC offset display string (e.g., "UTC-08:00")
    pub offset_str: String,
    /// DST badge information
    pub dst_badge: DstBadge,
    /// The second within the minute (0-59)
    pub second: u32,
    /// UTC offset in minutes (for overlap detection)
    pub utc_offset_minutes: i32,
}

impl LedgerEntry {
    /// Create a new ledger entry from a UTC instant and timezone
    pub fn from_instant(instant_utc: DateTime<Utc>, tz: Tz, is_dst: bool, in_overlap: bool, is_first_pass: bool) -> Self {
        let time_data = compute_time_data_at(tz, instant_utc);

        let local_timestamp = format!(
            "{:02}:{:02}:{:02} {}",
            time_data.hour12, time_data.minute, time_data.second, time_data.meridiem
        );

        // Determine DST badge
        let dst_badge = if in_overlap {
            if is_first_pass {
                DstBadge::OverlapPass1
            } else {
                DstBadge::OverlapPass2
            }
        } else if is_dst {
            DstBadge::Active
        } else {
            DstBadge::None
        };

        Self {
            instant_utc,
            local_timestamp,
            block_id: time_data.minute,
            chapter_id: time_data.hour24,
            offset_str: time_data.format_utc_offset(),
            dst_badge,
            second: time_data.second,
            utc_offset_minutes: time_data.utc_offset_minutes,
        }
    }

    /// Create a gap marker entry for DST spring forward
    pub fn gap_marker(instant_utc: DateTime<Utc>, from: String, to: String) -> Self {
        Self {
            instant_utc,
            local_timestamp: format!("{} → {}", from, to),
            block_id: 0,
            chapter_id: 0,
            offset_str: String::new(),
            dst_badge: DstBadge::GapMarker { from, to },
            second: 0,
            utc_offset_minutes: 0,
        }
    }

    /// Recalculate local timestamp for a new timezone
    pub fn recalculate_for_tz(&mut self, tz: Tz) {
        let time_data = compute_time_data_at(tz, self.instant_utc);

        self.local_timestamp = format!(
            "{:02}:{:02}:{:02} {}",
            time_data.hour12, time_data.minute, time_data.second, time_data.meridiem
        );
        self.block_id = time_data.minute;
        self.chapter_id = time_data.hour24;
        self.offset_str = time_data.format_utc_offset();
        self.utc_offset_minutes = time_data.utc_offset_minutes;

        // Update DST badge based on new timezone
        // Keep overlap markers as-is since they're based on UTC offset changes
        if !matches!(self.dst_badge, DstBadge::GapMarker { .. } | DstBadge::OverlapPass1 | DstBadge::OverlapPass2) {
            if time_data.is_dst {
                self.dst_badge = DstBadge::Active;
            } else {
                self.dst_badge = DstBadge::None;
            }
        }
    }

    /// Check if this entry is a special marker (gap or overlap)
    pub fn is_marker(&self) -> bool {
        matches!(
            self.dst_badge,
            DstBadge::GapMarker { .. } | DstBadge::OverlapPass1 | DstBadge::OverlapPass2
        )
    }
}

/// State for tracking DST fall-back overlap
#[derive(Debug, Clone, Default)]
struct OverlapState {
    /// Whether we're currently in a fall-back overlap period
    in_overlap: bool,
    /// The hour that's being repeated (in local time)
    overlap_hour: Option<u32>,
    /// Whether we're in the first pass (DST time) or second pass (standard time)
    is_first_pass: bool,
    /// The UTC offset before the fall-back (DST offset)
    pre_fallback_offset: Option<i32>,
}

/// State for the ledger view
#[derive(Debug)]
pub struct LedgerState {
    /// Rolling window of entries (newest at front)
    pub entries: VecDeque<LedgerEntry>,
    /// Set of collapsed blocks (hour, minute) tuples
    pub collapsed_blocks: HashSet<(u32, u32)>,
    /// Set of collapsed hour chapters
    pub collapsed_chapters: HashSet<u32>,
    /// Current scroll offset (0 = live/top)
    pub scroll_offset: f32,
    /// Whether we're in "live" mode (auto-scrolling to newest)
    pub is_live: bool,
    /// Current time range filter
    pub time_range: TimeRangeFilter,
    /// Last recorded second (to detect new seconds)
    last_second: Option<u32>,
    /// Last recorded minute (to detect minute boundaries)
    last_minute: Option<u32>,
    /// Last recorded UTC offset (for overlap detection)
    last_offset: Option<i32>,
    /// Fall-back overlap tracking state
    overlap_state: OverlapState,
}

impl Default for LedgerState {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            collapsed_blocks: HashSet::new(),
            collapsed_chapters: HashSet::new(),
            scroll_offset: 0.0,
            is_live: true,
            time_range: TimeRangeFilter::default(),
            last_second: None,
            last_minute: None,
            last_offset: None,
            overlap_state: OverlapState::default(),
        }
    }
}

impl LedgerState {
    /// Create a new ledger state
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the maximum number of entries based on current time range
    pub fn max_entries(&self) -> usize {
        self.time_range.as_seconds()
    }

    /// Update the ledger with new time data
    ///
    /// Returns true if a new entry was added
    pub fn update(&mut self, time_data: &TimeData, tz: Tz) -> bool {
        let current_second = time_data.second;

        // Only add entry if second changed
        if self.last_second == Some(current_second) {
            return false;
        }

        self.last_second = Some(current_second);

        // Check for DST transitions
        self.check_for_dst_transitions(time_data);

        // Create new entry with overlap awareness
        let entry = LedgerEntry::from_instant(
            time_data.local_datetime.with_timezone(&Utc),
            tz,
            time_data.is_dst,
            self.overlap_state.in_overlap,
            self.overlap_state.is_first_pass,
        );

        // Check for minute boundary (for potential DST gap detection)
        if let Some(last_min) = self.last_minute {
            if time_data.minute != last_min {
                self.check_for_dst_gap(time_data, tz);
            }
        }
        self.last_minute = Some(time_data.minute);
        self.last_offset = Some(time_data.utc_offset_minutes);

        // Add entry to front (newest first)
        self.entries.push_front(entry);

        // Prune old entries
        self.prune_entries();

        true
    }

    /// Check for DST transitions and update overlap state
    fn check_for_dst_transitions(&mut self, time_data: &TimeData) {
        match &time_data.dst_change {
            DstChange::JustOccurred { delta_minutes, .. } => {
                if *delta_minutes < 0 {
                    // Fall back just occurred - we're now in the second pass
                    // The overlap period started an hour ago
                    if self.overlap_state.in_overlap && self.overlap_state.is_first_pass {
                        // Transitioning from first pass to second pass
                        self.overlap_state.is_first_pass = false;
                    }
                }
            }
            DstChange::Upcoming { delta_minutes, .. } => {
                if *delta_minutes < 0 {
                    // Fall back is upcoming - we might be in the first pass of overlap
                    // Check if we're within the overlap hour
                    if let Some(last_offset) = self.last_offset {
                        if time_data.utc_offset_minutes != last_offset {
                            // Offset just changed - entering overlap period
                            self.overlap_state.in_overlap = true;
                            self.overlap_state.is_first_pass = true;
                            self.overlap_state.overlap_hour = Some(time_data.hour24);
                            self.overlap_state.pre_fallback_offset = Some(last_offset);
                        }
                    }
                }
            }
            DstChange::None => {
                // Check if we should exit overlap state
                if self.overlap_state.in_overlap {
                    // Exit overlap if we've moved past the overlap hour
                    if let Some(overlap_hour) = self.overlap_state.overlap_hour {
                        if time_data.hour24 != overlap_hour && !self.overlap_state.is_first_pass {
                            self.overlap_state = OverlapState::default();
                        }
                    }
                }
            }
        }

        // Detect fall-back by offset change (more reliable method)
        if let Some(last_offset) = self.last_offset {
            let offset_decreased = time_data.utc_offset_minutes < last_offset;
            if offset_decreased && !self.overlap_state.in_overlap {
                // We just fell back - mark that we're in second pass of overlap
                self.overlap_state.in_overlap = true;
                self.overlap_state.is_first_pass = false;
                self.overlap_state.overlap_hour = Some(time_data.hour24);
                self.overlap_state.pre_fallback_offset = Some(last_offset);

                // Retroactively mark recent entries in the same hour as Pass 1
                self.mark_overlap_pass1(time_data.hour24, last_offset);
            }
        }
    }

    /// Retroactively mark entries as Pass 1 when we detect a fall-back
    fn mark_overlap_pass1(&mut self, hour: u32, dst_offset: i32) {
        for entry in self.entries.iter_mut() {
            // Mark entries from the same hour that had the DST offset
            if entry.chapter_id == hour && entry.utc_offset_minutes == dst_offset {
                if !matches!(entry.dst_badge, DstBadge::GapMarker { .. }) {
                    entry.dst_badge = DstBadge::OverlapPass1;
                }
            }
        }
    }

    /// Check for DST gaps when minute changes
    fn check_for_dst_gap(&mut self, time_data: &TimeData, _tz: Tz) {
        // Check if we just had a DST spring forward
        if let DstChange::JustOccurred { delta_minutes, .. } = &time_data.dst_change {
            if *delta_minutes > 0 {
                // Spring forward - we skipped time
                // Insert a gap marker
                let skipped_hour = if time_data.hour24 > 0 {
                    time_data.hour24 - 1
                } else {
                    23
                };
                let gap_entry = LedgerEntry::gap_marker(
                    time_data.local_datetime.with_timezone(&Utc),
                    format!("{:02}:00", skipped_hour),
                    format!("{:02}:00", time_data.hour24),
                );
                self.entries.push_front(gap_entry);
            }
        }
    }

    /// Prune entries older than the window size
    fn prune_entries(&mut self) {
        let max = self.max_entries();
        while self.entries.len() > max {
            self.entries.pop_back();
        }
    }

    /// Recalculate all entries for a new timezone
    pub fn recalculate_for_tz(&mut self, tz: Tz) {
        for entry in self.entries.iter_mut() {
            entry.recalculate_for_tz(tz);
        }
    }

    /// Toggle collapse state for a block
    pub fn toggle_block_collapse(&mut self, hour: u32, minute: u32) {
        let key = (hour, minute);
        if self.collapsed_blocks.contains(&key) {
            self.collapsed_blocks.remove(&key);
        } else {
            self.collapsed_blocks.insert(key);
        }
    }

    /// Check if a block is collapsed
    pub fn is_block_collapsed(&self, hour: u32, minute: u32) -> bool {
        self.collapsed_blocks.contains(&(hour, minute))
    }

    /// Toggle collapse state for an hour chapter
    pub fn toggle_chapter_collapse(&mut self, hour: u32) {
        if self.collapsed_chapters.contains(&hour) {
            self.collapsed_chapters.remove(&hour);
        } else {
            self.collapsed_chapters.insert(hour);
        }
    }

    /// Check if an hour chapter is collapsed
    pub fn is_chapter_collapsed(&self, hour: u32) -> bool {
        self.collapsed_chapters.contains(&hour)
    }

    /// Collapse all visible blocks
    pub fn collapse_all(&mut self) {
        for entry in self.entries.iter() {
            let block_key = (entry.chapter_id, entry.block_id);
            self.collapsed_blocks.insert(block_key);
        }
    }

    /// Collapse all visible chapters
    pub fn collapse_all_chapters(&mut self) {
        for entry in self.entries.iter() {
            self.collapsed_chapters.insert(entry.chapter_id);
        }
    }

    /// Expand all blocks
    pub fn expand_all(&mut self) {
        self.collapsed_blocks.clear();
    }

    /// Expand all chapters
    pub fn expand_all_chapters(&mut self) {
        self.collapsed_chapters.clear();
    }

    /// Return to live mode (scroll to top)
    pub fn return_to_live(&mut self) {
        self.scroll_offset = 0.0;
        self.is_live = true;
    }

    /// Scroll by a delta amount
    pub fn scroll(&mut self, delta: f32) {
        self.scroll_offset = (self.scroll_offset + delta).max(0.0);

        // If scrolled away from top, exit live mode
        if self.scroll_offset > 0.5 {
            self.is_live = false;
        } else {
            self.scroll_offset = 0.0;
            self.is_live = true;
        }
    }

    /// Set the time range filter
    pub fn set_time_range(&mut self, range: TimeRangeFilter) {
        self.time_range = range;
        self.prune_entries();
    }

    /// Get entries grouped by hour chapters containing minute blocks
    pub fn get_chapter_grouped_entries(&self) -> Vec<HourChapter> {
        let mut chapters: Vec<HourChapter> = Vec::new();

        for entry in self.entries.iter() {
            // Find or create the chapter for this entry's hour
            let chapter_idx = chapters.iter().position(|c| c.hour == entry.chapter_id);

            let chapter = if let Some(idx) = chapter_idx {
                &mut chapters[idx]
            } else {
                // Create new chapter
                chapters.push(HourChapter {
                    hour: entry.chapter_id,
                    collapsed: self.is_chapter_collapsed(entry.chapter_id),
                    blocks: Vec::new(),
                });
                chapters.last_mut().unwrap()
            };

            // Find or create the block within this chapter
            let block_idx = chapter.blocks.iter().position(|b| b.minute == entry.block_id);

            if let Some(idx) = block_idx {
                chapter.blocks[idx].entries.push(entry.clone());
            } else {
                // Create new block
                chapter.blocks.push(BlockGroup {
                    hour: entry.chapter_id,
                    minute: entry.block_id,
                    collapsed: self.is_block_collapsed(entry.chapter_id, entry.block_id),
                    entries: vec![entry.clone()],
                });
            }
        }

        chapters
    }

    /// Get entries grouped by (hour, minute) blocks for display (flat view)
    pub fn get_grouped_entries(&self) -> Vec<BlockGroup> {
        let mut groups: Vec<BlockGroup> = Vec::new();

        for entry in self.entries.iter() {
            if let Some(last) = groups.last_mut() {
                if last.hour == entry.chapter_id && last.minute == entry.block_id {
                    last.entries.push(entry.clone());
                    continue;
                }
            }

            // New group
            groups.push(BlockGroup {
                hour: entry.chapter_id,
                minute: entry.block_id,
                collapsed: self.is_block_collapsed(entry.chapter_id, entry.block_id),
                entries: vec![entry.clone()],
            });
        }

        groups
    }
}

/// An hour chapter containing multiple minute blocks
#[derive(Debug, Clone)]
pub struct HourChapter {
    pub hour: u32,
    pub collapsed: bool,
    pub blocks: Vec<BlockGroup>,
}

impl HourChapter {
    /// Get the chapter header text
    pub fn header_text(&self) -> String {
        let total_entries: usize = self.blocks.iter().map(|b| b.entries.len()).sum();
        format!(
            "CHAPTER {:02} │ {} blocks │ {} entries",
            self.hour,
            self.blocks.len(),
            total_entries
        )
    }

    /// Format hour for 12-hour display
    pub fn hour_12(&self) -> (u32, &'static str) {
        match self.hour {
            0 => (12, "AM"),
            1..=11 => (self.hour, "AM"),
            12 => (12, "PM"),
            _ => (self.hour - 12, "PM"),
        }
    }
}

/// A group of entries for display (one minute block)
#[derive(Debug, Clone)]
pub struct BlockGroup {
    pub hour: u32,
    pub minute: u32,
    pub collapsed: bool,
    pub entries: Vec<LedgerEntry>,
}

impl BlockGroup {
    /// Get the block header text
    pub fn header_text(&self) -> String {
        format!(
            "BLOCK {:02} │ {} entries",
            self.minute,
            self.entries.len()
        )
    }
}
