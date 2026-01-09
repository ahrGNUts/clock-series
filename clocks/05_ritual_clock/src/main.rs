//! Ritual Clock
//!
//! A clock as an interactive ritual: the user "conducts" the passing of seconds
//! visually, while time remains authoritative. Features 12 "chorus nodes" (hours)
//! and 60 "beat nodes" (seconds) with gesture trails and animations.

mod drawing;
mod stage;
mod ui;

use std::time::Instant;

use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use shared::{compute_time_data, TimeData};

use crate::stage::StageGeometry;
use crate::ui::PickerState;

const CLOCK_NAME: &str = "ritual_clock";
const DEFAULT_TZ: &str = "America/Los_Angeles";
const CONDUCTOR_PANEL_HEIGHT: f32 = 120.0;

fn main() {
    nannou::app(model).update(update).run();
}

/// A point in the gesture trail
#[derive(Debug, Clone)]
pub struct TrailPoint {
    pub x: f32,
    pub y: f32,
    pub instant: Instant,
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
    /// Stage area (center)
    #[default]
    Stage,
    /// Conductor panel (bottom)
    ConductorPanel,
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_zone_id: String,
    favorites: Vec<String>,
    gesture_sensitivity: f32,
    overlay_always_on: bool,
    reduced_motion: bool,
    trails_enabled_in_reduced_motion: bool,
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
            gesture_sensitivity: 0.5,
            overlay_always_on: false,
            reduced_motion: false,
            trails_enabled_in_reduced_motion: false,
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
    /// Previous time data (for detecting second/minute boundaries)
    pub prev_second: u32,
    pub prev_minute: u32,

    /// Animation state
    pub beat_pulse_start: Option<Instant>,
    pub beat_pulse_index: usize,
    pub hour_shimmer_start: Option<Instant>,
    pub hour_shimmer_index: usize,

    /// Gesture trail points
    pub trail_points: Vec<TrailPoint>,
    pub is_pointer_down: bool,
    pub last_trail_sample: Option<Instant>,

    /// UI state
    pub gesture_sensitivity: f32,
    pub overlay_always_on: bool,
    pub overlay_visible: bool,
    pub overlay_last_interaction: Option<Instant>,
    pub highlighted_hour: Option<usize>,

    /// Reduced motion preference
    pub reduced_motion: bool,
    pub trails_enabled_in_reduced_motion: bool,

    /// Time zone switching animation
    pub retune_start: Option<Instant>,
    pub retune_delta_offset: i32,

    /// Picker state
    pub picker_state: PickerState,

    /// Focus region for keyboard navigation
    pub focus_region: FocusRegion,

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
    /// Set a new timezone with retune animation
    pub fn set_timezone(&mut self, tz: Tz) {
        let old_offset = self.time_data.utc_offset_minutes;
        self.selected_zone = tz;
        self.time_data = compute_time_data(tz);
        let new_offset = self.time_data.utc_offset_minutes;

        // Check validity
        if self.time_data.validity == shared::Validity::Ok {
            self.last_valid_zone = tz;
            self.tz_error = false;
        } else {
            // TZ data issue - show error
            self.tz_error = true;
            self.show_toast("Timezone data may be stale or missing".to_string());
        }

        // Trigger retune animation
        if !self.reduced_motion && old_offset != new_offset {
            self.retune_start = Some(Instant::now());
            self.retune_delta_offset = new_offset - old_offset;
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

    /// Cycle hour highlight
    pub fn cycle_hour_highlight(&mut self, delta: i32) {
        let current = self.highlighted_hour.unwrap_or(0) as i32;
        let new_hour = (current + delta).rem_euclid(12) as usize;
        self.highlighted_hour = Some(new_hour);
        self.trigger_overlay();
    }

    /// Toggle overlay always-on
    pub fn toggle_overlay_always_on(&mut self) {
        self.overlay_always_on = !self.overlay_always_on;
        if self.overlay_always_on {
            self.overlay_visible = true;
        }
        save_config(self);
    }

    /// Trigger overlay visibility (for interactions)
    pub fn trigger_overlay(&mut self) {
        self.overlay_visible = true;
        self.overlay_last_interaction = Some(Instant::now());
    }

    /// Update overlay fade
    pub fn update_overlay(&mut self) {
        if self.overlay_always_on {
            self.overlay_visible = true;
            return;
        }

        if let Some(last) = self.overlay_last_interaction {
            let elapsed = last.elapsed().as_secs_f32();
            if elapsed > 3.0 {
                self.overlay_visible = false;
            }
        }
    }

    /// Add a trail point (respecting sample rate limit)
    pub fn add_trail_point(&mut self, x: f32, y: f32) {
        let now = Instant::now();

        // Sample rate limit: max 60 samples/sec
        if let Some(last) = self.last_trail_sample {
            if now.duration_since(last).as_secs_f32() < 1.0 / 60.0 {
                return;
            }
        }

        self.last_trail_sample = Some(now);

        // Add point
        self.trail_points.push(TrailPoint { x, y, instant: now });

        // Cap at 256 points
        while self.trail_points.len() > 256 {
            self.trail_points.remove(0);
        }

        self.trigger_overlay();
    }

    /// Prune expired trail points
    pub fn prune_trail_points(&mut self) {
        let now = Instant::now();
        let lifetime = 2.0; // seconds
        self.trail_points
            .retain(|p| now.duration_since(p.instant).as_secs_f32() < lifetime);
    }

    /// Check if trails should be drawn
    pub fn should_draw_trails(&self) -> bool {
        if self.reduced_motion {
            self.trails_enabled_in_reduced_motion
        } else {
            true
        }
    }

    /// Cycle focus region
    pub fn cycle_focus_region(&mut self, reverse: bool) {
        self.focus_region = match (self.focus_region, reverse) {
            (FocusRegion::Stage, false) => FocusRegion::ConductorPanel,
            (FocusRegion::ConductorPanel, false) => FocusRegion::Stage,
            (FocusRegion::Stage, true) => FocusRegion::ConductorPanel,
            (FocusRegion::ConductorPanel, true) => FocusRegion::Stage,
        };
    }
}

fn save_config(model: &Model) {
    let config = Config {
        selected_zone_id: model.selected_zone.name().to_string(),
        favorites: model
            .favorites
            .iter()
            .map(|tz| tz.name().to_string())
            .collect(),
        gesture_sensitivity: model.gesture_sensitivity,
        overlay_always_on: model.overlay_always_on,
        reduced_motion: model.reduced_motion,
        trails_enabled_in_reduced_motion: model.trails_enabled_in_reduced_motion,
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
        .title("Ritual Clock")
        .size(1000, 800)
        .min_size(600, 500)
        .view(view)
        .key_pressed(key_pressed)
        .mouse_pressed(mouse_pressed)
        .mouse_released(mouse_released)
        .mouse_moved(mouse_moved)
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
    let prev_second = time_data.second;
    let prev_minute = time_data.minute;

    Model {
        selected_zone,
        favorites,
        time_data,
        prev_second,
        prev_minute,
        beat_pulse_start: None,
        beat_pulse_index: 0,
        hour_shimmer_start: None,
        hour_shimmer_index: 0,
        trail_points: Vec::new(),
        is_pointer_down: false,
        last_trail_sample: None,
        gesture_sensitivity: config.gesture_sensitivity,
        overlay_always_on: config.overlay_always_on,
        overlay_visible: config.overlay_always_on,
        overlay_last_interaction: None,
        highlighted_hour: None,
        reduced_motion: config.reduced_motion,
        trails_enabled_in_reduced_motion: config.trails_enabled_in_reduced_motion,
        retune_start: None,
        retune_delta_offset: 0,
        picker_state: PickerState::default(),
        focus_region: FocusRegion::default(),
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

    // Detect second boundary for beat pulse
    if model.time_data.second != model.prev_second {
        model.beat_pulse_start = Some(Instant::now());
        model.beat_pulse_index = model.time_data.second as usize;
        model.prev_second = model.time_data.second;
    }

    // Detect minute boundary for hour shimmer
    if model.time_data.minute != model.prev_minute && model.time_data.second == 0 {
        model.hour_shimmer_start = Some(Instant::now());
        // hIndex = (hour12 % 12) where 12 maps to 0
        model.hour_shimmer_index = (model.time_data.hour12 % 12) as usize;
        model.prev_minute = model.time_data.minute;
    }

    // Prune expired trail points
    model.prune_trail_points();

    // Prune expired toasts
    model.prune_toasts();

    // Update overlay fade
    model.update_overlay();

    // Begin egui frame
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();

    // Draw conductor panel UI
    let ui_result = ui::draw_conductor_panel(
        &ctx,
        &mut model.picker_state,
        model.selected_zone,
        &model.favorites,
        &model.time_data,
        &mut model.gesture_sensitivity,
        &mut model.overlay_always_on,
        &mut model.reduced_motion,
        &mut model.trails_enabled_in_reduced_motion,
    );

    drop(ctx);

    // Apply UI results
    if let Some(tz) = ui_result.set_timezone {
        model.set_timezone(tz);
    }
    if let Some(tz) = ui_result.toggle_favorite {
        model.toggle_favorite(tz);
    }
    if ui_result.sensitivity_changed {
        save_config(model);
    }
    if ui_result.overlay_changed {
        if model.overlay_always_on {
            model.overlay_visible = true;
        }
        save_config(model);
    }
    if ui_result.reduced_motion_changed {
        save_config(model);
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Calculate stage geometry
    let geometry = StageGeometry::calculate(window_rect, CONDUCTOR_PANEL_HEIGHT);

    // Draw background
    draw.background().color(drawing::colors::BACKGROUND);

    // Draw the stage (nodes and trails)
    drawing::draw_stage(
        &draw,
        &geometry,
        model,
    );

    // Draw digital overlay if visible
    if model.overlay_visible {
        drawing::draw_overlay(
            &draw,
            &geometry,
            &model.time_data,
            model.highlighted_hour,
            model.overlay_always_on,
        );
    }

    // Draw title
    draw.text("RITUAL CLOCK")
        .x_y(0.0, window_rect.top() - 25.0)
        .color(drawing::colors::TEXT_PRIMARY)
        .font_size(18)
        .w(300.0);

    // Draw error banner if TZ data issue
    if model.tz_error {
        drawing::draw_error_banner(&draw, window_rect);
    }

    // Draw toast notifications
    drawing::draw_toasts(&draw, window_rect, &model.toasts);

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

        // H - cycle hour highlights
        Key::H => {
            model.cycle_hour_highlight(1);
        }

        // S - toggle overlay always-on
        Key::S => {
            model.toggle_overlay_always_on();
        }

        // Arrow keys - cycle hour highlight when stage focused
        Key::Left => {
            if model.focus_region == FocusRegion::Stage {
                model.cycle_hour_highlight(-1);
            }
        }
        Key::Right => {
            if model.focus_region == FocusRegion::Stage {
                model.cycle_hour_highlight(1);
            }
        }

        // Tab - cycle focus regions
        Key::Tab => {
            if !model.picker_state.is_open {
                model.cycle_focus_region(mods.shift());
            }
        }

        // Escape - close picker or overlay
        Key::Escape => {
            if model.picker_state.is_open {
                model.picker_state.close();
            } else if model.overlay_visible && !model.overlay_always_on {
                model.overlay_visible = false;
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

        // Enter/Space - activate (for accessibility)
        Key::Return | Key::Space => {
            if model.focus_region == FocusRegion::Stage {
                if let Some(hour) = model.highlighted_hour {
                    model.highlighted_hour = Some(hour);
                    model.trigger_overlay();
                }
            }
        }

        _ => {}
    }
}

fn mouse_pressed(app: &App, model: &mut Model, button: MouseButton) {
    if button == MouseButton::Left {
        model.is_pointer_down = true;

        let pos = app.mouse.position();
        let window_rect = app.window_rect();
        let geometry = StageGeometry::calculate(window_rect, CONDUCTOR_PANEL_HEIGHT);

        // Check if clicking on an hour node
        if let Some(hour_idx) = geometry.hit_test_hour_node(pos.x, pos.y) {
            model.highlighted_hour = Some(hour_idx);
            model.trigger_overlay();
        }

        // Start trail
        model.add_trail_point(pos.x, pos.y);
    } else if button == MouseButton::Middle {
        // Rotary press: toggle overlay
        model.toggle_overlay_always_on();
    }
}

fn mouse_released(_app: &App, model: &mut Model, button: MouseButton) {
    if button == MouseButton::Left {
        model.is_pointer_down = false;
    }
}

fn mouse_moved(_app: &App, model: &mut Model, pos: Point2) {
    if model.is_pointer_down && model.should_draw_trails() {
        model.add_trail_point(pos.x, pos.y);
    }
}

fn mouse_wheel(_app: &App, model: &mut Model, delta: MouseScrollDelta, _phase: TouchPhase) {
    // Rotary input: cycle hour highlights or adjust sensitivity
    let scroll_y = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
    };

    if model.focus_region == FocusRegion::ConductorPanel {
        // Adjust sensitivity
        model.gesture_sensitivity = (model.gesture_sensitivity + scroll_y * 0.1).clamp(0.0, 1.0);
        save_config(model);
    } else {
        // Cycle hour highlight
        if scroll_y > 0.5 {
            model.cycle_hour_highlight(-1);
        } else if scroll_y < -0.5 {
            model.cycle_hour_highlight(1);
        }
    }
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
                model.prev_second = model.time_data.second;
                model.prev_minute = model.time_data.minute;
            }
        }
        // Handle touch events (map to mouse-like behavior)
        nannou::winit::event::WindowEvent::Touch(touch) => {
            let window_rect = app.window_rect();
            let geometry = StageGeometry::calculate(window_rect, CONDUCTOR_PANEL_HEIGHT);

            // Convert touch position to nannou coordinates
            let pos_x = touch.location.x as f32 - window_rect.w() / 2.0;
            let pos_y = window_rect.h() / 2.0 - touch.location.y as f32;

            match touch.phase {
                nannou::winit::event::TouchPhase::Started => {
                    model.is_pointer_down = true;

                    // Check if touching an hour node
                    if let Some(hour_idx) = geometry.hit_test_hour_node(pos_x, pos_y) {
                        model.highlighted_hour = Some(hour_idx);
                        model.trigger_overlay();
                    }

                    // Start trail
                    if model.should_draw_trails() {
                        model.add_trail_point(pos_x, pos_y);
                    }
                }
                nannou::winit::event::TouchPhase::Moved => {
                    if model.is_pointer_down && model.should_draw_trails() {
                        model.add_trail_point(pos_x, pos_y);
                    }
                }
                nannou::winit::event::TouchPhase::Ended | nannou::winit::event::TouchPhase::Cancelled => {
                    model.is_pointer_down = false;
                }
            }
        }
        _ => {}
    }
}

