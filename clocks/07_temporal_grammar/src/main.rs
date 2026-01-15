//! Temporal Grammar Clock
//!
//! A clock as a semantic field: time is expressed as relationships and
//! transformations rather than digits. The primary experience is a living
//! diagram whose geometry encodes hour/minute/second and whose topology
//! encodes time zone + DST.

mod drawing;
mod geometry;
mod ui;

use std::time::Instant;

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use shared::{compute_time_data, compute_time_data_at, TimeData, Validity};

use crate::geometry::{
    apply_tz_transform, apply_tz_transform_minute_layer, apply_view_transform_points,
    compute_dst_knot, compute_geometry_params, compute_hour_polygon, compute_phase_ring,
    compute_superellipse, generate_diagram_description, GeometryParams, PhaseRing,
};
use crate::ui::PickerState;

const CLOCK_NAME: &str = "temporal_grammar";
const DEFAULT_TZ: &str = "America/Los_Angeles";
const SIDEBAR_WIDTH: f32 = 260.0;
const TOUCH_HOLD_THRESHOLD_MS: u128 = 350;

fn main() {
    nannou::app(model).update(update).run();
}

/// Toast notification for transient messages
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
            ((self.duration_secs - elapsed) / 0.5).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

/// Focus region for keyboard navigation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FocusRegion {
    /// No explicit focus (initial state)
    #[default]
    None,
    /// TZ control
    TzControl,
    /// DST status
    DstStatus,
    /// Truth Anchor hint
    TruthAnchorHint,
    /// Canvas (main diagram area)
    Canvas,
    /// Sidebar
    Sidebar,
}

impl FocusRegion {
    fn next(self) -> Self {
        match self {
            FocusRegion::None => FocusRegion::TzControl,
            FocusRegion::TzControl => FocusRegion::DstStatus,
            FocusRegion::DstStatus => FocusRegion::TruthAnchorHint,
            FocusRegion::TruthAnchorHint => FocusRegion::Canvas,
            FocusRegion::Canvas => FocusRegion::Sidebar,
            FocusRegion::Sidebar => FocusRegion::TzControl,
        }
    }

    fn prev(self) -> Self {
        match self {
            FocusRegion::None => FocusRegion::Sidebar,
            FocusRegion::TzControl => FocusRegion::Sidebar,
            FocusRegion::DstStatus => FocusRegion::TzControl,
            FocusRegion::TruthAnchorHint => FocusRegion::DstStatus,
            FocusRegion::Canvas => FocusRegion::TruthAnchorHint,
            FocusRegion::Sidebar => FocusRegion::Canvas,
        }
    }
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_zone_id: String,
    favorites: Vec<String>,
    decode_mode: bool,
    explicit_mode: bool,
    reduced_motion: bool,
    view_zoom: f32,
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
            decode_mode: false,
            explicit_mode: false,
            reduced_motion: false,
            view_zoom: 1.0,
        }
    }
}

/// Application state
pub struct Model {
    // Time state
    pub selected_zone: Tz,
    pub favorites: Vec<Tz>,
    pub time_data: TimeData,

    // Time manipulation
    pub is_live: bool,
    pub manual_time: DateTime<Utc>,

    // View state (pan/zoom)
    pub view_offset: Vec2,
    pub view_zoom: f32,
    pub is_panning: bool,
    pub last_mouse_pos: Point2,

    // Interaction state
    pub truth_anchor_active: bool,
    pub truth_anchor_latched: bool, // For rotary input
    pub truth_anchor_position: Option<Point2>,
    pub mouse_press_start: Option<Instant>,
    pub touch_start_time: Option<Instant>,
    pub space_held: bool,
    pub decode_mode: bool,
    pub explicit_mode: bool,
    pub help_panel_open: bool,

    // Computed geometry
    pub geometry_params: GeometryParams,
    pub hour_polygon: Vec<Point2>,
    pub minute_superellipse: Vec<Point2>,
    pub phase_ring: PhaseRing,
    pub diagram_description: String,

    // Accessibility
    pub reduced_motion: bool,

    // UI state
    pub picker_state: PickerState,
    pub focus_region: FocusRegion,
    pub window_focused: bool,

    // Toast notifications
    pub toasts: Vec<Toast>,

    // Error state
    pub tz_error: bool,
    pub last_valid_zone: Tz,

    // egui integration
    egui: Egui,
}

impl Model {
    /// Set a new timezone
    pub fn set_timezone(&mut self, tz: Tz) {
        let old_zone = self.selected_zone;
        self.selected_zone = tz;
        self.time_data = compute_time_data(tz);

        if self.time_data.validity == Validity::Ok {
            self.last_valid_zone = tz;
            self.tz_error = false;
        } else {
            self.tz_error = true;
            self.show_toast("Timezone data may be stale or missing".to_string());
        }

        if old_zone != tz {
            self.recompute_geometry();
        }

        save_config(self);
    }

    /// Try to set a timezone from a string
    pub fn try_set_timezone(&mut self, tz_str: &str) {
        match tz_str.parse::<Tz>() {
            Ok(tz) => self.set_timezone(tz),
            Err(_) => {
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

    /// Cycle focus region
    pub fn cycle_focus(&mut self, reverse: bool) {
        self.focus_region = if reverse {
            self.focus_region.prev()
        } else {
            self.focus_region.next()
        };
    }

    /// Activate the Truth Anchor (show time overlay)
    pub fn activate_truth_anchor(&mut self, position: Option<Point2>) {
        self.truth_anchor_active = true;
        self.truth_anchor_position = position;
    }

    /// Deactivate the Truth Anchor
    pub fn deactivate_truth_anchor(&mut self) {
        if !self.truth_anchor_latched {
            self.truth_anchor_active = false;
            self.truth_anchor_position = None;
        }
    }

    /// Toggle Truth Anchor latch (for rotary input)
    pub fn toggle_truth_anchor_latch(&mut self) {
        self.truth_anchor_latched = !self.truth_anchor_latched;
        self.truth_anchor_active = self.truth_anchor_latched;
        if !self.truth_anchor_latched {
            self.truth_anchor_position = None;
        }
    }

    /// Step time forward or backward
    pub fn step_time(&mut self, seconds: i64) {
        if self.is_live {
            // Switch to manual mode, starting from current time
            self.is_live = false;
            self.manual_time = Utc::now();
        }
        self.manual_time = self.manual_time + Duration::seconds(seconds);
        self.time_data = compute_time_data_at(self.selected_zone, self.manual_time);
        self.recompute_geometry();
    }

    /// Return to live time
    pub fn return_to_live(&mut self) {
        self.is_live = true;
        self.time_data = compute_time_data(self.selected_zone);
        self.recompute_geometry();
    }

    /// Recompute all geometry based on current time data
    pub fn recompute_geometry(&mut self) {
        let center = pt2(0.0, 0.0);
        let min_dim = 600.0; // Base dimension, will be scaled by view

        // Compute geometry parameters
        self.geometry_params = compute_geometry_params(
            self.time_data.hour12,
            self.time_data.minute,
            self.time_data.second,
            self.time_data.utc_offset_minutes,
            self.time_data.is_dst,
        );

        // Compute hour polygon
        let raw_polygon = compute_hour_polygon(self.time_data.hour12, min_dim, center);
        self.hour_polygon = apply_tz_transform(
            &raw_polygon,
            self.time_data.utc_offset_minutes,
            self.time_data.is_dst,
            center,
        );

        // Compute minute superellipse
        let raw_superellipse = compute_superellipse(self.time_data.minute, min_dim, center, 256);
        self.minute_superellipse = apply_tz_transform_minute_layer(
            &raw_superellipse,
            self.time_data.utc_offset_minutes,
            self.time_data.is_dst,
            center,
        );

        // Compute phase ring
        self.phase_ring = compute_phase_ring(
            self.time_data.second,
            self.time_data.second_fraction,
            min_dim,
            center,
            self.reduced_motion,
        );

        // Generate description
        self.diagram_description = generate_diagram_description(
            &self.geometry_params,
            self.selected_zone.name(),
        );
    }

    /// Apply pan delta
    pub fn pan(&mut self, delta: Vec2) {
        self.view_offset += delta;
    }

    /// Apply zoom
    pub fn zoom(&mut self, factor: f32) {
        self.view_zoom = (self.view_zoom * factor).clamp(0.3, 3.0);
        save_config(self);
    }

    /// Reset view to default
    pub fn reset_view(&mut self) {
        self.view_offset = vec2(0.0, 0.0);
        self.view_zoom = 1.0;
        save_config(self);
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
        decode_mode: model.decode_mode,
        explicit_mode: model.explicit_mode,
        reduced_motion: model.reduced_motion,
        view_zoom: model.view_zoom,
    };
    if let Err(e) = shared::save_config(CLOCK_NAME, &config) {
        eprintln!("Failed to save config: {}", e);
    }
}

fn model(app: &App) -> Model {
    app.set_exit_on_escape(false);

    let window_id = app
        .new_window()
        .title("Temporal Grammar Clock")
        .size(1000, 750)
        .min_size(700, 500)
        .view(view)
        .key_pressed(key_pressed)
        .key_released(key_released)
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

    // Initialize geometry (will be recomputed in first update)
    let center = pt2(0.0, 0.0);
    let min_dim = 600.0;

    let geometry_params = compute_geometry_params(
        time_data.hour12,
        time_data.minute,
        time_data.second,
        time_data.utc_offset_minutes,
        time_data.is_dst,
    );

    let raw_polygon = compute_hour_polygon(time_data.hour12, min_dim, center);
    let hour_polygon = apply_tz_transform(
        &raw_polygon,
        time_data.utc_offset_minutes,
        time_data.is_dst,
        center,
    );

    let raw_superellipse = compute_superellipse(time_data.minute, min_dim, center, 256);
    let minute_superellipse = apply_tz_transform_minute_layer(
        &raw_superellipse,
        time_data.utc_offset_minutes,
        time_data.is_dst,
        center,
    );

    let phase_ring = compute_phase_ring(
        time_data.second,
        time_data.second_fraction,
        min_dim,
        center,
        config.reduced_motion,
    );

    let diagram_description = generate_diagram_description(&geometry_params, selected_zone.name());

    Model {
        selected_zone,
        favorites,
        time_data,
        is_live: true,
        manual_time: Utc::now(),
        view_offset: vec2(0.0, 0.0),
        view_zoom: config.view_zoom,
        is_panning: false,
        last_mouse_pos: pt2(0.0, 0.0),
        truth_anchor_active: false,
        truth_anchor_latched: false,
        truth_anchor_position: None,
        mouse_press_start: None,
        touch_start_time: None,
        space_held: false,
        decode_mode: config.decode_mode,
        explicit_mode: config.explicit_mode,
        help_panel_open: false,
        geometry_params,
        hour_polygon,
        minute_superellipse,
        phase_ring,
        diagram_description,
        reduced_motion: config.reduced_motion,
        picker_state: PickerState::default(),
        focus_region: FocusRegion::default(),
        window_focused: true,
        toasts: Vec::new(),
        tz_error: false,
        last_valid_zone: selected_zone,
        egui,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    // Update time data only when in live mode
    if model.is_live {
        model.time_data = compute_time_data(model.selected_zone);
        model.recompute_geometry();
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
        model.decode_mode,
        model.explicit_mode,
        model.reduced_motion,
        &model.diagram_description,
        model.is_live,
    );

    drop(ctx);

    // Apply UI results
    if let Some(tz) = ui_result.set_timezone {
        model.set_timezone(tz);
    }
    if let Some(tz) = ui_result.toggle_favorite {
        model.toggle_favorite(tz);
    }
    if ui_result.toggle_decode_mode {
        model.decode_mode = !model.decode_mode;
        save_config(model);
    }
    if ui_result.toggle_explicit_mode {
        model.explicit_mode = !model.explicit_mode;
        save_config(model);
    }
    if ui_result.toggle_reduced_motion {
        model.reduced_motion = !model.reduced_motion;
        save_config(model);
    }
    if ui_result.open_help {
        model.help_panel_open = true;
    }
    if let Some(seconds) = ui_result.step_time {
        model.step_time(seconds);
    }
    if ui_result.return_to_live {
        model.return_to_live();
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Draw background
    draw.background().color(drawing::colors::BACKGROUND);

    // Calculate canvas area (excluding sidebar on the right)
    let canvas_width = window_rect.w() - SIDEBAR_WIDTH;
    let canvas_rect = Rect::from_x_y_w_h(
        window_rect.left() + canvas_width / 2.0,
        window_rect.y(),
        canvas_width,
        window_rect.h(),
    );

    let center = canvas_rect.xy();

    if model.explicit_mode {
        // Draw explicit mode (standard time readout)
        drawing::draw_explicit_mode(&draw, &model.time_data, canvas_rect, model.selected_zone.name());
    } else {
        // Apply view transform to geometry
        let transformed_polygon = apply_view_transform_points(
            &model.hour_polygon,
            model.view_offset,
            model.view_zoom,
            pt2(0.0, 0.0),
        );
        let transformed_superellipse = apply_view_transform_points(
            &model.minute_superellipse,
            model.view_offset,
            model.view_zoom,
            pt2(0.0, 0.0),
        );

        // Create transformed phase ring
        let transformed_marks = apply_view_transform_points(
            &model.phase_ring.marks,
            model.view_offset,
            model.view_zoom,
            pt2(0.0, 0.0),
        );
        let transformed_center = geometry::apply_view_transform(
            model.phase_ring.center,
            model.view_offset,
            model.view_zoom,
            pt2(0.0, 0.0),
        );
        let transformed_ring = PhaseRing {
            center: transformed_center + center,
            radius: model.phase_ring.radius * model.view_zoom,
            marks: transformed_marks.iter().map(|p| *p + center).collect(),
            highlighted_index: model.phase_ring.highlighted_index,
            needle_angle: model.phase_ring.needle_angle,
        };

        // Offset all geometry by canvas center
        let polygon_centered: Vec<Point2> = transformed_polygon.iter().map(|p| *p + center).collect();
        let superellipse_centered: Vec<Point2> = transformed_superellipse.iter().map(|p| *p + center).collect();

        // Draw layers in order: foundation, tension, phase
        drawing::draw_foundation_layer(&draw, &polygon_centered);
        drawing::draw_tension_layer(&draw, &superellipse_centered);
        drawing::draw_phase_layer(&draw, &transformed_ring, model.view_zoom);

        // Draw DST knot if applicable
        if let Some(knot) = compute_dst_knot(
            &model.time_data.dst_change,
            model.time_data.utc_offset_minutes,
            model.time_data.is_dst,
            600.0 * model.view_zoom,
            center + model.view_offset,
            Utc::now(),
        ) {
            drawing::draw_dst_knot(&draw, &knot);
        }

        // Draw decode mode guides
        if model.decode_mode {
            drawing::draw_decode_mode_guides(
                &draw,
                &model.geometry_params,
                &transformed_ring,
                center + model.view_offset,
                canvas_rect,
            );
        }
    }

    // Draw HUD elements
    drawing::draw_hud(
        &draw,
        window_rect,
        model.time_data.is_dst,
        &model.time_data.dst_change,
        !model.truth_anchor_active,
    );

    // Draw focus indicator if canvas is focused
    if model.focus_region == FocusRegion::Canvas {
        drawing::draw_focus_indicator(&draw, canvas_rect);
    }

    // Draw Truth Anchor overlay
    if model.truth_anchor_active {
        let overlay_pos = model
            .truth_anchor_position
            .unwrap_or(canvas_rect.xy());
        drawing::draw_truth_anchor_overlay(
            &draw,
            &model.time_data,
            overlay_pos,
            model.selected_zone.name(),
        );
    }

    // Draw help panel (centered on canvas area, not whole window)
    if model.help_panel_open {
        drawing::draw_help_panel(&draw, canvas_rect);
    }

    // Draw error banner if TZ data issue
    if model.tz_error {
        drawing::draw_error_banner(&draw, window_rect);
    }

    // Draw toast notifications
    for toast in &model.toasts {
        drawing::draw_toast(&draw, &toast.message, toast.alpha(), window_rect);
    }

    // Render to frame
    draw.to_frame(app, &frame).unwrap();

    // Render egui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    let mods = app.keys.mods;

    match key {
        // Space - activate Truth Anchor (hold)
        Key::Space => {
            if !model.space_held {
                model.space_held = true;
                model.activate_truth_anchor(None);
            }
        }

        // D - toggle Decode Mode
        Key::D => {
            if !model.picker_state.is_open && !model.help_panel_open {
                model.decode_mode = !model.decode_mode;
                save_config(model);
            }
        }

        // Z - open timezone picker
        Key::Z => {
            if !model.help_panel_open {
                model.picker_state.open();
            }
        }

        // ? (Shift + /) - toggle help panel
        Key::Slash if mods.shift() => {
            model.help_panel_open = !model.help_panel_open;
        }

        // Tab - cycle focus
        Key::Tab => {
            if !model.picker_state.is_open && !model.help_panel_open {
                model.cycle_focus(mods.shift());
            }
        }

        // Escape - close panels
        Key::Escape => {
            if model.help_panel_open {
                model.help_panel_open = false;
            } else if model.picker_state.is_open {
                model.picker_state.close();
            } else if model.truth_anchor_latched {
                model.truth_anchor_latched = false;
                model.deactivate_truth_anchor();
            }
        }

        // Arrow keys - pan when canvas focused
        Key::Up => {
            if model.focus_region == FocusRegion::Canvas {
                model.pan(vec2(0.0, 20.0));
            }
        }
        Key::Down => {
            if model.focus_region == FocusRegion::Canvas {
                model.pan(vec2(0.0, -20.0));
            }
        }
        Key::Left => {
            if model.focus_region == FocusRegion::Canvas {
                model.pan(vec2(-20.0, 0.0));
            }
        }
        Key::Right => {
            if model.focus_region == FocusRegion::Canvas {
                model.pan(vec2(20.0, 0.0));
            }
        }

        // R - reset view
        Key::R => {
            if model.focus_region == FocusRegion::Canvas {
                model.reset_view();
            }
        }

        // + / = - zoom in
        Key::Equals | Key::Plus => {
            if model.focus_region == FocusRegion::Canvas {
                model.zoom(1.1);
            }
        }

        // - - zoom out
        Key::Minus => {
            if model.focus_region == FocusRegion::Canvas {
                model.zoom(0.9);
            }
        }

        // L - return to live time
        Key::L => {
            if !model.picker_state.is_open && !model.help_panel_open {
                model.return_to_live();
            }
        }

        // [ - step backward in time
        Key::LBracket => {
            if !model.picker_state.is_open && !model.help_panel_open {
                if mods.ctrl() || mods.logo() {
                    model.step_time(-3600); // -1 hour
                } else if mods.shift() {
                    model.step_time(-60); // -1 minute
                } else {
                    model.step_time(-1); // -1 second
                }
            }
        }

        // ] - step forward in time
        Key::RBracket => {
            if !model.picker_state.is_open && !model.help_panel_open {
                if mods.ctrl() || mods.logo() {
                    model.step_time(3600); // +1 hour
                } else if mods.shift() {
                    model.step_time(60); // +1 minute
                } else {
                    model.step_time(1); // +1 second
                }
            }
        }

        _ => {}
    }
}

fn key_released(_app: &App, model: &mut Model, key: Key) {
    match key {
        Key::Space => {
            model.space_held = false;
            model.deactivate_truth_anchor();
        }
        _ => {}
    }
}

fn mouse_pressed(app: &App, model: &mut Model, button: MouseButton) {
    let pos = app.mouse.position();
    let window_rect = app.window_rect();

    // Calculate canvas area
    let canvas_rect = Rect::from_x_y_w_h(
        window_rect.left() + (window_rect.w() - SIDEBAR_WIDTH) / 2.0 - SIDEBAR_WIDTH / 2.0,
        window_rect.y(),
        window_rect.w() - SIDEBAR_WIDTH,
        window_rect.h(),
    );

    match button {
        MouseButton::Left => {
            if canvas_rect.contains(pt2(pos.x, pos.y)) {
                // Start Truth Anchor on press
                model.mouse_press_start = Some(Instant::now());
                model.activate_truth_anchor(Some(pos));

                // Also start panning
                model.is_panning = true;
                model.last_mouse_pos = pos;
            }
        }
        MouseButton::Middle => {
            // Rotary press equivalent - toggle latch
            model.toggle_truth_anchor_latch();
        }
        _ => {}
    }
}

fn mouse_released(_app: &App, model: &mut Model, button: MouseButton) {
    match button {
        MouseButton::Left => {
            model.is_panning = false;
            model.mouse_press_start = None;
            model.deactivate_truth_anchor();
        }
        _ => {}
    }
}

fn mouse_moved(_app: &App, model: &mut Model, pos: Point2) {
    if model.is_panning {
        let delta = pos - model.last_mouse_pos;
        model.pan(delta);
        model.last_mouse_pos = pos;

        // Update Truth Anchor position while dragging
        if model.truth_anchor_active {
            model.truth_anchor_position = Some(pos);
        }
    }

    // Update last mouse position for future reference
    model.last_mouse_pos = pos;
}

fn mouse_wheel(_app: &App, model: &mut Model, delta: MouseScrollDelta, _phase: TouchPhase) {
    let scroll_amount = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
    };

    if scroll_amount > 0.0 {
        model.zoom(1.1);
    } else if scroll_amount < 0.0 {
        model.zoom(0.9);
    }
}

fn raw_window_event(app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);

    match event {
        nannou::winit::event::WindowEvent::Focused(focused) => {
            model.window_focused = *focused;
            if *focused {
                // Resync time on focus
                model.time_data = compute_time_data(model.selected_zone);
                model.recompute_geometry();
            }
        }
        nannou::winit::event::WindowEvent::Touch(touch) => {
            let window_rect = app.window_rect();

            // Convert touch position to nannou coordinates
            let pos_x = touch.location.x as f32 - window_rect.w() / 2.0;
            let pos_y = window_rect.h() / 2.0 - touch.location.y as f32;
            let pos = pt2(pos_x, pos_y);

            match touch.phase {
                nannou::winit::event::TouchPhase::Started => {
                    model.touch_start_time = Some(Instant::now());
                    model.last_mouse_pos = pos;
                }
                nannou::winit::event::TouchPhase::Moved => {
                    // Check if this is a long press (350ms)
                    if let Some(start) = model.touch_start_time {
                        if start.elapsed().as_millis() >= TOUCH_HOLD_THRESHOLD_MS {
                            model.activate_truth_anchor(Some(pos));
                        }
                    }

                    // Pan
                    let delta = pos - model.last_mouse_pos;
                    model.pan(delta);
                    model.last_mouse_pos = pos;
                }
                nannou::winit::event::TouchPhase::Ended | nannou::winit::event::TouchPhase::Cancelled => {
                    model.touch_start_time = None;
                    model.deactivate_truth_anchor();
                }
            }
        }
        _ => {}
    }
}

