//! Temporal Topography Clock
//!
//! A clock as a topographic map of the day where elevations represent
//! "temporal intensity." You read time by locating yourself on the terrain.

mod drawing;
mod terrain;
mod ui;

use chrono::Utc;
use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use shared::{compute_time_data, compute_time_data_at, TimeData};

use crate::drawing::{
    colors, draw_day_map, draw_help_hints, draw_hover_tooltip, draw_inspect_cursor, draw_title,
    MapLayout,
};
use crate::terrain::{DayDomain, HourBoundary, TerrainParams, generate_hour_boundaries};
use crate::ui::{
    draw_side_panel, draw_timezone_picker, PickerResult, PickerState, SidePanelResult,
};

const CLOCK_NAME: &str = "temporal_topography";
const DEFAULT_TZ: &str = "America/Los_Angeles";
const SIDE_PANEL_WIDTH: f32 = 280.0;

fn main() {
    nannou::app(model).update(update).run();
}

/// Application mode
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    /// Live mode - beacon at current time
    Live,
    /// Inspecting mode - user is exploring a specific point on the map
    Inspecting {
        /// Normalized position [0..1] being inspected
        inspect_position: f32,
        /// Whether the inspection point is pinned
        is_pinned: bool,
    },
}

impl Mode {
    fn is_inspecting(&self) -> bool {
        matches!(self, Mode::Inspecting { .. })
    }

    fn inspect_position(&self) -> Option<f32> {
        match self {
            Mode::Inspecting { inspect_position, .. } => Some(*inspect_position),
            Mode::Live => None,
        }
    }

    #[allow(dead_code)]
    fn is_pinned(&self) -> bool {
        match self {
            Mode::Inspecting { is_pinned, .. } => *is_pinned,
            Mode::Live => false,
        }
    }
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_tz_id: String,
    favorites: Vec<String>,
    reduced_motion: bool,
    show_legend: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            selected_tz_id: DEFAULT_TZ.to_string(),
            favorites: vec![
                "America/New_York".to_string(),
                "America/Los_Angeles".to_string(),
                "Europe/London".to_string(),
                "Asia/Tokyo".to_string(),
            ],
            reduced_motion: false,
            show_legend: true,
        }
    }
}

/// Application state
struct Model {
    /// Current mode (Live or Inspecting)
    mode: Mode,
    /// Current time data
    time_data: TimeData,
    /// Selected timezone
    selected_tz: Tz,
    /// Favorite timezones
    favorites: Vec<Tz>,
    /// Timezone picker state
    picker_state: PickerState,
    /// Reduced motion preference
    reduced_motion: bool,
    /// Whether to show the legend
    show_legend: bool,
    /// Current day domain (cached)
    day_domain: DayDomain,
    /// Hour boundaries for grid (cached)
    hour_boundaries: Vec<HourBoundary>,
    /// Terrain parameters (cached)
    terrain_params: TerrainParams,
    /// Mouse position for hover inspection
    mouse_position: Option<Point2>,
    /// Last click time for double-click detection
    last_click_time: Option<std::time::Instant>,
    /// egui integration
    egui: Egui,
}

impl Model {
    fn enter_inspect(&mut self, position: f32) {
        // Snap to nearest minute boundary
        let snapped = self.day_domain.snap_to_minute(position);
        self.mode = Mode::Inspecting {
            inspect_position: snapped.clamp(0.0, 1.0),
            is_pinned: false,
        };
    }

    fn return_to_live(&mut self) {
        self.mode = Mode::Live;
    }

    fn toggle_pin(&mut self) {
        if let Mode::Inspecting { inspect_position, is_pinned } = &self.mode {
            self.mode = Mode::Inspecting {
                inspect_position: *inspect_position,
                is_pinned: !*is_pinned,
            };
        }
    }

    fn adjust_inspect(&mut self, delta_minutes: i64) {
        match &self.mode {
            Mode::Live => {
                // Enter inspect mode at current position (snapped), then adjust
                let position = self.day_domain.snap_to_minute(self.day_domain.normalized_position);
                let ssm = self.day_domain.position_to_ssm(position);
                let new_ssm = ssm + delta_minutes * 60;
                let new_position = self.day_domain.ssm_to_position(new_ssm);
                self.mode = Mode::Inspecting {
                    inspect_position: new_position.clamp(0.0, 1.0),
                    is_pinned: false,
                };
            }
            Mode::Inspecting { inspect_position, is_pinned } => {
                let ssm = self.day_domain.position_to_ssm(*inspect_position);
                let new_ssm = ssm + delta_minutes * 60;
                let new_position = self.day_domain.ssm_to_position(new_ssm);
                self.mode = Mode::Inspecting {
                    inspect_position: new_position.clamp(0.0, 1.0),
                    is_pinned: *is_pinned,
                };
            }
        }
    }

    /// Format time at a given normalized position
    fn format_time_at_position(&self, position: f32) -> String {
        let ssm = self.day_domain.position_to_ssm(position);
        let hours = (ssm / 3600) % 24;
        let minutes = (ssm % 3600) / 60;
        let seconds = ssm % 60;

        let hour12 = match hours {
            0 => 12,
            1..=12 => hours,
            _ => hours - 12,
        };
        let meridiem = if hours < 12 { "AM" } else { "PM" };

        format!("{}:{:02}:{:02} {}", hour12, minutes, seconds, meridiem)
    }

    /// Check if a position is in a DST gap
    fn is_position_in_gap(&self, position: f32) -> bool {
        self.day_domain.is_in_gap(position)
    }

    /// Check if a position is in a DST overlap
    fn is_position_in_overlap(&self, position: f32) -> bool {
        self.day_domain.is_in_overlap(position).is_some()
    }
}

fn save_config(model: &Model) {
    let config = Config {
        selected_tz_id: model.selected_tz.name().to_string(),
        favorites: model
            .favorites
            .iter()
            .map(|tz| tz.name().to_string())
            .collect(),
        reduced_motion: model.reduced_motion,
        show_legend: model.show_legend,
    };
    if let Err(e) = shared::save_config(CLOCK_NAME, &config) {
        eprintln!("Failed to save config: {}", e);
    }
}

fn toggle_favorite(favorites: &mut Vec<Tz>, tz: Tz) {
    if let Some(pos) = favorites.iter().position(|&t| t == tz) {
        favorites.remove(pos);
    } else {
        favorites.push(tz);
    }
}

fn model(app: &App) -> Model {
    // Create window
    let window_id = app
        .new_window()
        .title("Temporal Topography")
        .size(1200, 700)
        .min_size(900, 600)
        .view(view)
        .key_pressed(key_pressed)
        .mouse_pressed(mouse_pressed)
        .mouse_moved(mouse_moved)
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
    let selected_tz: Tz = config
        .selected_tz_id
        .parse()
        .unwrap_or_else(|_| DEFAULT_TZ.parse().unwrap());

    // Parse favorite timezones
    let favorites: Vec<Tz> = config
        .favorites
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    // Compute initial time data
    let now = Utc::now();
    let time_data = compute_time_data(selected_tz);
    let day_domain = DayDomain::compute(now, selected_tz);
    let hour_boundaries = generate_hour_boundaries(selected_tz, &day_domain);
    let terrain_params = TerrainParams::from_datetime(time_data.local_datetime);

    Model {
        mode: Mode::Live,
        time_data,
        selected_tz,
        favorites,
        picker_state: PickerState::default(),
        reduced_motion: config.reduced_motion,
        show_legend: config.show_legend,
        day_domain,
        hour_boundaries,
        terrain_params,
        mouse_position: None,
        last_click_time: None,
        egui,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let now = Utc::now();

    // Update time data based on mode
    let display_instant = match &model.mode {
        Mode::Live => now,
        Mode::Inspecting { inspect_position, .. } => {
            // Calculate the instant for the inspected position
            let ssm = model.day_domain.position_to_ssm(*inspect_position);
            model.day_domain.midnight_utc + chrono::Duration::seconds(ssm)
        }
    };

    model.time_data = compute_time_data_at(model.selected_tz, display_instant);

    // Always update day domain based on current time (for proper day boundaries)
    let new_day_domain = DayDomain::compute(now, model.selected_tz);

    // Check if day changed (regenerate hour boundaries)
    if new_day_domain.midnight_utc != model.day_domain.midnight_utc {
        model.day_domain = new_day_domain;
        model.hour_boundaries = generate_hour_boundaries(model.selected_tz, &model.day_domain);
    } else {
        // Just update the normalized position
        model.day_domain = new_day_domain;
    }

    // Update terrain params
    model.terrain_params = TerrainParams::from_datetime(model.time_data.local_datetime);

    // Collect UI state before borrowing egui
    let current_tz = model.selected_tz;
    let favorites_clone = model.favorites.clone();
    let time_data_clone = model.time_data.clone();
    let is_inspecting = model.mode.is_inspecting();
    let mut reduced_motion = model.reduced_motion;
    let mut show_legend = model.show_legend;

    // Get inspect info if in inspect mode (before borrowing egui)
    let inspect_time_str = model
        .mode
        .inspect_position()
        .map(|p| model.format_time_at_position(p));
    
    let inspect_is_gap = model
        .mode
        .inspect_position()
        .map(|p| model.is_position_in_gap(p))
        .unwrap_or(false);
    
    let inspect_is_overlap = model
        .mode
        .inspect_position()
        .map(|p| model.is_position_in_overlap(p))
        .unwrap_or(false);

    // Begin egui frame
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();

    // Draw side panel
    let panel_result: SidePanelResult = draw_side_panel(
        &ctx,
        &time_data_clone,
        is_inspecting,
        inspect_time_str.as_deref(),
        inspect_is_gap,
        inspect_is_overlap,
        &mut reduced_motion,
        &mut show_legend,
    );

    // Draw timezone picker (if open)
    let picker_result: PickerResult = draw_timezone_picker(
        &ctx,
        &mut model.picker_state,
        current_tz,
        &favorites_clone,
    );

    // Apply results
    drop(ctx);

    // Handle panel result
    if panel_result.open_picker {
        model.picker_state.open();
    }
    if panel_result.return_to_now {
        model.return_to_live();
    }
    if panel_result.reduced_motion_changed {
        model.reduced_motion = reduced_motion;
        save_config(model);
    }
    if panel_result.legend_toggled {
        model.show_legend = show_legend;
        save_config(model);
    }

    // Handle picker result
    if let Some(tz) = picker_result.selected_tz {
        model.selected_tz = tz;
        model.time_data = compute_time_data(tz);
        // Regenerate day domain and hour boundaries
        model.day_domain = DayDomain::compute(now, tz);
        model.hour_boundaries = generate_hour_boundaries(tz, &model.day_domain);
        save_config(model);
    }
    if let Some(tz) = picker_result.toggle_favorite {
        toggle_favorite(&mut model.favorites, tz);
        save_config(model);
    }
    if picker_result.close_picker {
        model.picker_state.close();
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Clear background
    draw.background().color(colors::BACKGROUND);

    // Calculate layout
    let layout = MapLayout::calculate(window_rect, SIDE_PANEL_WIDTH);

    // Get time fraction for beacon pulse animation
    let time_fraction = model.time_data.second_fraction as f32;

    // Draw the day map
    draw_day_map(
        &draw,
        &layout,
        &model.terrain_params,
        &model.day_domain,
        &model.hour_boundaries,
        model.reduced_motion,
        time_fraction,
    );

    // Draw inspect cursor if in inspect mode
    if let Mode::Inspecting { inspect_position, is_pinned } = &model.mode {
        draw_inspect_cursor(&draw, &layout, *inspect_position, *is_pinned);
    }

    // Draw hover tooltip when mouse is over map (and not in pinned inspect mode)
    let is_pinned = matches!(&model.mode, Mode::Inspecting { is_pinned: true, .. });
    if !is_pinned {
        if let Some(mouse_pos) = model.mouse_position {
            if layout.contains(mouse_pos.x, mouse_pos.y) {
                let hover_position = layout.x_to_position(mouse_pos.x);
                let hover_time_str = model.format_time_at_position(hover_position);
                draw_hover_tooltip(&draw, &layout, mouse_pos.x, mouse_pos.y, &hover_time_str);
            }
        }
    }

    // Draw title
    draw_title(&draw, window_rect);

    // Draw help hints
    draw_help_hints(&draw, &layout, window_rect);

    // Render to frame
    draw.to_frame(app, &frame).unwrap();

    // Render egui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    let mods = app.keys.mods;

    match key {
        // Escape - close picker or return to live
        Key::Escape => {
            if model.picker_state.is_open {
                model.picker_state.close();
            } else if model.mode.is_inspecting() {
                model.return_to_live();
            }
        }

        // Enter/Space - toggle pin in inspect mode
        Key::Return | Key::Space => {
            if !model.picker_state.is_open && model.mode.is_inspecting() {
                model.toggle_pin();
            }
        }

        // Slash - focus search / open picker
        Key::Slash => {
            if !model.picker_state.is_open {
                model.picker_state.open();
            } else {
                model.picker_state.should_focus_search = true;
            }
        }

        // Arrow keys - step inspection cursor
        Key::Left => {
            if mods.shift() {
                model.adjust_inspect(-60); // -1 hour
            } else {
                model.adjust_inspect(-1); // -1 minute
            }
        }
        Key::Right => {
            if mods.shift() {
                model.adjust_inspect(60); // +1 hour
            } else {
                model.adjust_inspect(1); // +1 minute
            }
        }

        // R - toggle reduced motion
        Key::R => {
            if !model.picker_state.is_open {
                model.reduced_motion = !model.reduced_motion;
                save_config(model);
            }
        }

        _ => {}
    }
}

fn mouse_pressed(app: &App, model: &mut Model, button: MouseButton) {
    if button == MouseButton::Left && !model.picker_state.is_open {
        let mouse_pos = app.mouse.position();
        let window_rect = app.window_rect();
        let layout = MapLayout::calculate(window_rect, SIDE_PANEL_WIDTH);

        // Check if click is within the map canvas
        if layout.contains(mouse_pos.x, mouse_pos.y) {
            let now = std::time::Instant::now();
            
            // Check for double-click (within 300ms)
            if let Some(last_click) = model.last_click_time {
                if now.duration_since(last_click).as_millis() < 300 {
                    // Double-click detected - return to live mode
                    model.return_to_live();
                    model.last_click_time = None;
                    return;
                }
            }
            
            model.last_click_time = Some(now);
            
            let position = layout.x_to_position(mouse_pos.x);

            // If already inspecting this position, toggle pin
            if let Mode::Inspecting { inspect_position, .. } = &model.mode {
                if (*inspect_position - position).abs() < 0.01 {
                    model.toggle_pin();
                    return;
                }
            }

            model.enter_inspect(position);
        }
    }
}

fn mouse_moved(app: &App, model: &mut Model, pos: Point2) {
    model.mouse_position = Some(pos);

    // If in unpinned inspect mode, follow the mouse (snapped to minute)
    if let Mode::Inspecting { is_pinned: false, .. } = &model.mode {
        let window_rect = app.window_rect();
        let layout = MapLayout::calculate(window_rect, SIDE_PANEL_WIDTH);

        if layout.contains(pos.x, pos.y) {
            let position = layout.x_to_position(pos.x);
            let snapped = model.day_domain.snap_to_minute(position);
            model.mode = Mode::Inspecting {
                inspect_position: snapped,
                is_pinned: false,
            };
        }
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    // Let egui handle raw events
    model.egui.handle_raw_event(event);
}

