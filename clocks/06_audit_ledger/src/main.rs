//! Audit Ledger Clock
//!
//! A clock as an event ledger: each second is an entry; minutes are blocks;
//! hours are chapters. Features terminal/console aesthetic with cryptographic
//! hash verification stamps.

mod drawing;
mod ledger;
mod ui;

use std::time::Instant;

use chrono::Utc;
use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::{compute_time_data, TimeData, Validity};

use crate::ledger::{LedgerState, TimeRangeFilter};
use crate::ui::PickerState;

const CLOCK_NAME: &str = "audit_ledger";
const DEFAULT_TZ: &str = "America/Los_Angeles";
const SIDEBAR_WIDTH: f32 = 280.0;

fn main() {
    nannou::app(model).update(update).run();
}

/// Toast notification for error messages
#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub created: Instant,
    pub duration_secs: f32,
}

impl Toast {
    pub fn new(message: String, duration_secs: f32) -> Self {
        Self {
            message,
            created: Instant::now(),
            duration_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created.elapsed().as_secs_f32() > self.duration_secs
    }

    pub fn alpha(&self) -> f32 {
        let elapsed = self.created.elapsed().as_secs_f32();
        if elapsed > self.duration_secs - 0.5 {
            // Fade out in last 0.5s
            ((self.duration_secs - elapsed) / 0.5).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

/// Focus region for keyboard navigation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FocusRegion {
    /// Ledger view (main area)
    #[default]
    Ledger,
    /// Sidebar controls
    Sidebar,
}

/// Text density for accessibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextDensity {
    Compact,
    #[default]
    Normal,
    Large,
}

impl TextDensity {
    pub fn row_height(&self) -> f32 {
        match self {
            TextDensity::Compact => 18.0,
            TextDensity::Normal => 24.0,
            TextDensity::Large => 32.0,
        }
    }

    pub fn font_size(&self) -> u32 {
        match self {
            TextDensity::Compact => 12,
            TextDensity::Normal => 14,
            TextDensity::Large => 18,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TextDensity::Compact => "Compact",
            TextDensity::Normal => "Normal",
            TextDensity::Large => "Large",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            TextDensity::Compact => TextDensity::Normal,
            TextDensity::Normal => TextDensity::Large,
            TextDensity::Large => TextDensity::Compact,
        }
    }
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_zone_id: String,
    favorites: Vec<String>,
    time_range_minutes: u32,
    text_density: TextDensity,
    reduced_motion: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            selected_zone_id: DEFAULT_TZ.to_string(),
            favorites: vec![
                "America/New_York".to_string(),
                "America/Los_Angeles".to_string(),
                "Europe/London".to_string(),
                "Asia/Tokyo".to_string(),
            ],
            time_range_minutes: 10,
            text_density: TextDensity::Normal,
            reduced_motion: false,
        }
    }
}

/// Application state
pub struct Model {
    /// Selected time zone
    pub selected_zone: Tz,
    /// Favorite time zones
    pub favorites: Vec<Tz>,
    /// Current time data
    pub time_data: TimeData,

    /// Ledger state
    pub ledger: LedgerState,

    /// Current verification hash (truncated)
    pub verification_hash: String,

    /// UI state
    pub text_density: TextDensity,
    pub reduced_motion: bool,

    /// Timezone switching animation
    pub relabel_start: Option<Instant>,
    pub relabel_progress: f32,

    /// Picker state
    pub picker_state: PickerState,

    /// Focus region for keyboard navigation
    pub focus_region: FocusRegion,
    /// Focused block index (for keyboard navigation)
    pub focused_block_index: Option<usize>,

    /// Window focus state
    pub window_focused: bool,

    /// Animation time (seconds since start)
    pub animation_time: f32,

    /// Toast notifications
    pub toasts: Vec<Toast>,

    /// Error state: TZ data validity
    pub tz_error: bool,

    /// Last valid timezone (for fallback)
    pub last_valid_zone: Tz,

    /// egui integration
    egui: Egui,
}

impl Model {
    /// Set a new timezone with relabel animation
    pub fn set_timezone(&mut self, tz: Tz) {
        let old_zone = self.selected_zone;
        self.selected_zone = tz;
        self.time_data = compute_time_data(tz);

        // Check validity
        if self.time_data.validity == Validity::Ok {
            self.last_valid_zone = tz;
            self.tz_error = false;
        } else {
            // TZ data issue - show error
            self.tz_error = true;
            self.show_toast("Timezone data may be stale or missing".to_string());
        }

        // Trigger relabel animation if timezone actually changed
        if old_zone != tz {
            if !self.reduced_motion {
                self.relabel_start = Some(Instant::now());
                self.relabel_progress = 0.0;
            }

            // Recalculate all ledger entries for new timezone
            self.ledger.recalculate_for_tz(tz);
        }

        save_config(self);
    }

    /// Try to set a timezone from a string, with error handling
    pub fn try_set_timezone(&mut self, tz_str: &str) {
        match tz_str.parse::<Tz>() {
            Ok(tz) => self.set_timezone(tz),
            Err(_) => {
                // Invalid TZ - revert to last valid and show toast
                self.show_toast(format!("Invalid timezone: {}. Reverting.", tz_str));
                self.selected_zone = self.last_valid_zone;
                self.time_data = compute_time_data(self.last_valid_zone);
            }
        }
    }

    /// Show a toast notification
    pub fn show_toast(&mut self, message: String) {
        self.toasts.push(Toast::new(message, 4.0));
    }

    /// Prune expired toasts
    pub fn prune_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }

    /// Toggle favorite status for a zone
    pub fn toggle_favorite(&mut self, tz: Tz) {
        if let Some(pos) = self.favorites.iter().position(|&t| t == tz) {
            self.favorites.remove(pos);
        } else {
            self.favorites.push(tz);
        }
        save_config(self);
    }

    /// Compute the verification hash for the current time
    pub fn compute_verification_hash(&mut self) {
        let now_utc = Utc::now();
        let input = format!(
            "{}|{}",
            now_utc.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            self.selected_zone.name()
        );

        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();

        // Truncate to 16 hex chars
        self.verification_hash = hex::encode(&result[..8]).to_uppercase();
    }

    /// Cycle focus region
    pub fn cycle_focus_region(&mut self, reverse: bool) {
        self.focus_region = match (self.focus_region, reverse) {
            (FocusRegion::Ledger, false) => FocusRegion::Sidebar,
            (FocusRegion::Sidebar, false) => FocusRegion::Ledger,
            (FocusRegion::Ledger, true) => FocusRegion::Sidebar,
            (FocusRegion::Sidebar, true) => FocusRegion::Ledger,
        };
    }

    /// Navigate to next/previous block
    pub fn navigate_block(&mut self, delta: i32) {
        let groups = self.ledger.get_grouped_entries();
        if groups.is_empty() {
            self.focused_block_index = None;
            return;
        }

        let current = self.focused_block_index.unwrap_or(0) as i32;
        let new_index = (current + delta).clamp(0, groups.len() as i32 - 1) as usize;
        self.focused_block_index = Some(new_index);
    }

    /// Toggle collapse on focused block
    pub fn toggle_focused_block(&mut self) {
        if let Some(idx) = self.focused_block_index {
            let groups = self.ledger.get_grouped_entries();
            if let Some(group) = groups.get(idx) {
                self.ledger.toggle_block_collapse(group.hour, group.minute);
            }
        }
    }

    /// Toggle collapse on the chapter containing the focused block
    pub fn toggle_focused_chapter(&mut self) {
        if let Some(idx) = self.focused_block_index {
            let groups = self.ledger.get_grouped_entries();
            if let Some(group) = groups.get(idx) {
                self.ledger.toggle_chapter_collapse(group.hour);
            }
        }
    }

    /// Set text density
    pub fn set_text_density(&mut self, density: TextDensity) {
        self.text_density = density;
        save_config(self);
    }

    /// Set reduced motion preference
    pub fn set_reduced_motion(&mut self, enabled: bool) {
        self.reduced_motion = enabled;
        save_config(self);
    }

    /// Set time range filter
    pub fn set_time_range(&mut self, range: TimeRangeFilter) {
        self.ledger.set_time_range(range);
        save_config(self);
    }
}

fn save_config(model: &Model) {
    let time_range_minutes = match model.ledger.time_range {
        TimeRangeFilter::Minutes5 => 5,
        TimeRangeFilter::Minutes10 => 10,
        TimeRangeFilter::Minutes30 => 30,
        TimeRangeFilter::Minutes60 => 60,
    };

    let config = Config {
        selected_zone_id: model.selected_zone.name().to_string(),
        favorites: model
            .favorites
            .iter()
            .map(|tz| tz.name().to_string())
            .collect(),
        time_range_minutes,
        text_density: model.text_density,
        reduced_motion: model.reduced_motion,
    };
    if let Err(e) = shared::save_config(CLOCK_NAME, &config) {
        eprintln!("Failed to save config: {}", e);
    }
}

fn model(app: &App) -> Model {
    // Disable default escape-to-exit behavior
    app.set_exit_on_escape(false);

    // Create window
    let window_id = app
        .new_window()
        .title("Audit Ledger Clock")
        .size(1100, 800)
        .min_size(800, 600)
        .view(view)
        .key_pressed(key_pressed)
        .mouse_pressed(mouse_pressed)
        .mouse_wheel(mouse_wheel)
        .raw_event(raw_window_event)
        .build()
        .unwrap();

    let window = app.window(window_id).unwrap();
    let egui = Egui::from_window(&window);

    // Load configuration
    let config: Config = shared::load_config(CLOCK_NAME)
        .ok()
        .flatten()
        .unwrap_or_default();

    // Parse timezone from config
    let selected_zone: Tz = config
        .selected_zone_id
        .parse()
        .unwrap_or_else(|_| DEFAULT_TZ.parse().unwrap());

    let favorites: Vec<Tz> = config
        .favorites
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    // Get initial time data
    let time_data = compute_time_data(selected_zone);

    // Set up ledger with configured time range
    let mut ledger = LedgerState::new();
    let time_range = match config.time_range_minutes {
        5 => TimeRangeFilter::Minutes5,
        30 => TimeRangeFilter::Minutes30,
        60 => TimeRangeFilter::Minutes60,
        _ => TimeRangeFilter::Minutes10,
    };
    ledger.set_time_range(time_range);

    // Compute initial hash
    let now_utc = Utc::now();
    let input = format!(
        "{}|{}",
        now_utc.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
        selected_zone.name()
    );
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let verification_hash = hex::encode(&result[..8]).to_uppercase();

    Model {
        selected_zone,
        favorites,
        time_data,
        ledger,
        verification_hash,
        text_density: config.text_density,
        reduced_motion: config.reduced_motion,
        relabel_start: None,
        relabel_progress: 0.0,
        picker_state: PickerState::default(),
        focus_region: FocusRegion::default(),
        focused_block_index: None,
        window_focused: true,
        animation_time: 0.0,
        toasts: Vec::new(),
        tz_error: false,
        last_valid_zone: selected_zone,
        egui,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    // Update animation time
    model.animation_time = update.since_start.as_secs_f32();

    // Update time data
    model.time_data = compute_time_data(model.selected_zone);

    // Update ledger with new time data
    model.ledger.update(&model.time_data, model.selected_zone);

    // Update verification hash
    model.compute_verification_hash();

    // Update relabel animation
    if let Some(start) = model.relabel_start {
        let elapsed = start.elapsed().as_secs_f32();
        let duration = 0.3; // 300ms

        if elapsed < duration {
            model.relabel_progress = elapsed / duration;
        } else {
            model.relabel_start = None;
            model.relabel_progress = 1.0;
        }
    }

    // Prune expired toasts
    model.prune_toasts();

    // Begin egui frame
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();

    // Draw sidebar UI
    let ui_result = ui::draw_sidebar(
        &ctx,
        &mut model.picker_state,
        model.selected_zone,
        &model.favorites,
        &model.time_data,
        &model.ledger,
        model.text_density,
        model.reduced_motion,
    );

    drop(ctx);

    // Apply UI results
    if let Some(tz) = ui_result.set_timezone {
        model.set_timezone(tz);
    }
    if let Some(tz) = ui_result.toggle_favorite {
        model.toggle_favorite(tz);
    }
    if let Some(range) = ui_result.set_time_range {
        model.set_time_range(range);
    }
    if let Some(density) = ui_result.set_density {
        model.set_text_density(density);
    }
    if let Some(reduced) = ui_result.set_reduced_motion {
        model.set_reduced_motion(reduced);
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Draw background
    draw.background().color(drawing::colors::BACKGROUND);

    // Calculate layout regions
    let ledger_rect = Rect::from_x_y_w_h(
        window_rect.left() + (window_rect.w() - SIDEBAR_WIDTH) / 2.0,
        window_rect.y(),
        window_rect.w() - SIDEBAR_WIDTH,
        window_rect.h(),
    );

    // Draw header
    drawing::draw_header(&draw, &ledger_rect, &model.time_data, &model.verification_hash);

    // Draw ledger
    drawing::draw_ledger(
        &draw,
        &ledger_rect,
        model,
    );

    // Draw "Return to Live" button if not live
    if !model.ledger.is_live {
        drawing::draw_return_to_live_button(&draw, &ledger_rect);
    }

    // Draw error banner if TZ data issue
    if model.tz_error {
        drawing::draw_error_banner(&draw, window_rect);
    }

    // Draw toast notifications
    drawing::draw_toasts(&draw, window_rect, &model.toasts);

    // Draw focus indicator
    if model.focus_region == FocusRegion::Ledger {
        drawing::draw_focus_indicator(&draw, &ledger_rect);
    }

    // Render to frame
    draw.to_frame(app, &frame).unwrap();

    // Render egui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    let mods = app.keys.mods;

    match key {
        // T - open timezone picker
        Key::T => {
            if !model.picker_state.is_open {
                model.picker_state.open();
            }
        }

        // / - focus search in picker
        Key::Slash => {
            if model.picker_state.is_open {
                model.picker_state.should_focus_search = true;
            } else {
                model.picker_state.open();
            }
        }

        // L - return to live
        Key::L => {
            model.ledger.return_to_live();
        }

        // J/K or Down/Up - scroll ledger
        Key::J | Key::Down => {
            if model.focus_region == FocusRegion::Ledger {
                model.ledger.scroll(model.text_density.row_height() * 3.0);
                model.navigate_block(1);
            }
        }
        Key::K | Key::Up => {
            if model.focus_region == FocusRegion::Ledger {
                model.ledger.scroll(-model.text_density.row_height() * 3.0);
                model.navigate_block(-1);
            }
        }

        // [ - collapse blocks/chapters
        Key::LBracket => {
            if mods.shift() {
                // Shift+[ = collapse all chapters
                model.ledger.collapse_all_chapters();
            } else if mods.ctrl() || mods.logo() {
                // Ctrl+[ = collapse all blocks
                model.ledger.collapse_all();
            } else {
                // [ = toggle focused block
                model.toggle_focused_block();
            }
        }

        // ] - expand blocks/chapters
        Key::RBracket => {
            if mods.shift() {
                // Shift+] = expand all chapters
                model.ledger.expand_all_chapters();
            } else if mods.ctrl() || mods.logo() {
                // Ctrl+] = expand all blocks
                model.ledger.expand_all();
            } else {
                // ] = toggle focused block
                model.toggle_focused_block();
            }
        }

        // C - toggle focused chapter collapse
        Key::C => {
            if model.focus_region == FocusRegion::Ledger {
                model.toggle_focused_chapter();
            }
        }

        // Tab - cycle focus regions
        Key::Tab => {
            if !model.picker_state.is_open {
                model.cycle_focus_region(mods.shift());
            }
        }

        // Enter/Space - activate focused element
        Key::Return | Key::Space => {
            if model.focus_region == FocusRegion::Ledger {
                model.toggle_focused_block();
            }
        }

        // Escape - close picker or return to live
        Key::Escape => {
            if model.picker_state.is_open {
                model.picker_state.close();
            } else if !model.ledger.is_live {
                model.ledger.return_to_live();
            }
        }

        _ => {}
    }
}

fn mouse_pressed(app: &App, model: &mut Model, button: MouseButton) {
    if button == MouseButton::Left {
        let pos = app.mouse.position();
        let window_rect = app.window_rect();

        // Calculate ledger rect
        let ledger_rect = Rect::from_x_y_w_h(
            window_rect.left() + (window_rect.w() - SIDEBAR_WIDTH) / 2.0,
            window_rect.y(),
            window_rect.w() - SIDEBAR_WIDTH,
            window_rect.h(),
        );

        // Check if clicking "Return to Live" button
        if !model.ledger.is_live {
            let button_rect = Rect::from_x_y_w_h(
                ledger_rect.x(),
                ledger_rect.bottom() + 60.0,
                200.0,
                40.0,
            );
            if button_rect.contains(pt2(pos.x, pos.y)) {
                model.ledger.return_to_live();
                return;
            }
        }

        // Check if clicking in ledger area
        if ledger_rect.contains(pt2(pos.x, pos.y)) {
            model.focus_region = FocusRegion::Ledger;

            // Hit test for block headers
            let groups = model.ledger.get_grouped_entries();
            let header_height = 30.0;
            let row_height = model.text_density.row_height();
            let start_y = ledger_rect.top() - 80.0 - model.ledger.scroll_offset;
            let mut current_y = start_y;

            for (idx, group) in groups.iter().enumerate() {
                let header_rect = Rect::from_x_y_w_h(
                    ledger_rect.x(),
                    current_y - header_height / 2.0,
                    ledger_rect.w() - 40.0,
                    header_height,
                );

                if header_rect.contains(pt2(pos.x, pos.y)) {
                    model.focused_block_index = Some(idx);
                    model.ledger.toggle_block_collapse(group.hour, group.minute);
                    return;
                }

                current_y -= header_height;

                if !group.collapsed {
                    current_y -= row_height * group.entries.len() as f32;
                }
            }
        }
    } else if button == MouseButton::Middle {
        // Rotary press: toggle collapse on focused block
        model.toggle_focused_block();
    }
}

fn mouse_wheel(_app: &App, model: &mut Model, delta: MouseScrollDelta, _phase: TouchPhase) {
    let scroll_amount = match delta {
        MouseScrollDelta::LineDelta(_, y) => y * model.text_density.row_height() * 3.0,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
    };

    model.ledger.scroll(-scroll_amount);
}

fn raw_window_event(app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);

    // Track window focus for resync
    match event {
        nannou::winit::event::WindowEvent::Focused(focused) => {
            model.window_focused = *focused;
            if *focused {
                // Resync time on focus
                model.time_data = compute_time_data(model.selected_zone);
            }
        }
        // Handle touch events
        nannou::winit::event::WindowEvent::Touch(touch) => {
            let window_rect = app.window_rect();

            // Convert touch position to nannou coordinates
            let pos_x = touch.location.x as f32 - window_rect.w() / 2.0;
            let pos_y = window_rect.h() / 2.0 - touch.location.y as f32;

            match touch.phase {
                nannou::winit::event::TouchPhase::Started => {
                    // Check for "Return to Live" tap
                    if !model.ledger.is_live {
                        let ledger_rect = Rect::from_x_y_w_h(
                            window_rect.left() + (window_rect.w() - SIDEBAR_WIDTH) / 2.0,
                            window_rect.y(),
                            window_rect.w() - SIDEBAR_WIDTH,
                            window_rect.h(),
                        );
                        let button_rect = Rect::from_x_y_w_h(
                            ledger_rect.x(),
                            ledger_rect.bottom() + 60.0,
                            200.0,
                            40.0,
                        );
                        if button_rect.contains(pt2(pos_x, pos_y)) {
                            model.ledger.return_to_live();
                        }
                    }
                }
                nannou::winit::event::TouchPhase::Moved => {
                    // Could implement swipe-to-scroll here
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// Hex encoding helper (since we need it for the hash)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

