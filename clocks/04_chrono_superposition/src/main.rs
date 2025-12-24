//! Chrono-Superposition Clock
//!
//! A clock that treats time zones as simultaneous realities. Multiple time zones
//! are shown at once in a superposed "deck," which collapses into a composite
//! readout when focused.

mod cards;
mod drawing;
mod ui;

use std::collections::HashMap;

use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use shared::{compute_time_data, TimeData};

use crate::cards::{compute_display_order, CardGeometry};
use crate::drawing::{colors, draw_card_deck, draw_composite_readout, draw_list_view, CoreLayout};
use crate::ui::{
    draw_collapse_controls, draw_zone_field, CollapseControlsResult, PickerState,
    ZoneFieldResult,
};

const CLOCK_NAME: &str = "chrono_superposition";
const DEFAULT_TZ: &str = "America/Los_Angeles";
const LEFT_PANEL_WIDTH: f32 = 240.0;
const RIGHT_PANEL_WIDTH: f32 = 200.0;

fn main() {
    nannou::app(model).update(update).run();
}

/// View state machine
#[derive(Debug, Clone, PartialEq)]
pub enum ViewState {
    /// Cards spread in deck formation
    DeckView,
    /// Cards collapsed into composite readout (focus_strength >= 0.8)
    CompositeView,
    /// Accessible list view
    ListView,
    /// Timezone picker is open
    PickerOpen,
}

/// Focus region for keyboard navigation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FocusRegion {
    /// Zone Field panel (left)
    ZoneField,
    /// Core deck area (center)
    #[default]
    CoreDeck,
    /// Collapse Controls panel (right)
    CollapseControls,
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_zone_ids: Vec<String>,
    dominant_zone_id: String,
    favorites: Vec<String>,
    focus_strength: f32,
    compare_mode: bool,
    list_mode: bool,
    list_mode_override: bool,
    reduced_motion: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            selected_zone_ids: vec![
                "America/Los_Angeles".to_string(),
                "America/New_York".to_string(),
                "Europe/London".to_string(),
                "Asia/Tokyo".to_string(),
            ],
            dominant_zone_id: DEFAULT_TZ.to_string(),
            favorites: vec![
                "America/New_York".to_string(),
                "America/Los_Angeles".to_string(),
                "Europe/London".to_string(),
                "Asia/Tokyo".to_string(),
            ],
            focus_strength: 0.0,
            compare_mode: false,
            list_mode: false,
            list_mode_override: false,
            reduced_motion: false,
        }
    }
}

/// Application state
pub struct Model {
    /// Selected time zones (1..N)
    pub selected_zones: Vec<Tz>,
    /// The dominant (top) zone
    pub dominant_zone: Tz,
    /// Favorite time zones
    pub favorites: Vec<Tz>,
    /// Cached time data per zone
    pub zone_times: HashMap<Tz, TimeData>,
    /// Display order (computed each frame)
    pub display_order: Vec<Tz>,

    /// Focus strength slider (0.0 = spread, 1.0 = collapsed)
    pub focus_strength: f32,
    /// Whether compare mode is active
    pub compare_mode: bool,
    /// Whether list mode is active (accessibility)
    pub list_mode: bool,
    /// Whether list mode was manually overridden
    pub list_mode_override: bool,

    /// Current view state
    pub view_state: ViewState,

    /// Mouse position for parallax
    pub mouse_position: Option<Point2>,
    /// Window center for parallax calculation
    pub window_center: Point2,
    /// Index of hovered card (if any)
    pub hovered_card_index: Option<usize>,

    /// Timezone picker state
    pub picker_state: PickerState,
    /// Reduced motion preference
    pub reduced_motion: bool,
    /// Animation time for pulsing effects
    pub animation_time: f32,

    /// Current focus region for keyboard navigation
    pub focus_region: FocusRegion,

    /// egui integration
    egui: Egui,
}

impl Model {
    /// Set a new dominant zone
    pub fn set_dominant(&mut self, tz: Tz) {
        if self.selected_zones.contains(&tz) {
            self.dominant_zone = tz;
            self.update_display_order();
            save_config(self);
        }
    }

    /// Add a zone to selected zones
    pub fn add_zone(&mut self, tz: Tz) {
        if !self.selected_zones.contains(&tz) {
            self.selected_zones.push(tz);
            self.update_display_order();
            self.check_list_mode_threshold();
            save_config(self);
        }
    }

    /// Remove a zone from selected zones
    pub fn remove_zone(&mut self, tz: Tz) {
        if self.selected_zones.len() > 1 {
            self.selected_zones.retain(|&z| z != tz);
            // If we removed the dominant zone, pick a new one
            if self.dominant_zone == tz {
                self.dominant_zone = self.selected_zones[0];
            }
            self.update_display_order();
            self.check_list_mode_threshold();
            save_config(self);
        }
    }

    /// Toggle favorite status for a zone
    pub fn toggle_favorite(&mut self, tz: Tz) {
        if let Some(pos) = self.favorites.iter().position(|&t| t == tz) {
            self.favorites.remove(pos);
        } else {
            self.favorites.push(tz);
        }
        self.update_display_order();
        save_config(self);
    }

    /// Cycle dominance up/down in display order
    pub fn cycle_dominance(&mut self, delta: i32) {
        if self.display_order.len() <= 1 {
            return;
        }
        let current_idx = self
            .display_order
            .iter()
            .position(|&z| z == self.dominant_zone)
            .unwrap_or(0);
        let new_idx = (current_idx as i32 + delta)
            .rem_euclid(self.display_order.len() as i32) as usize;
        self.dominant_zone = self.display_order[new_idx];
        self.update_display_order();
        save_config(self);
    }

    /// Toggle compare mode
    pub fn toggle_compare_mode(&mut self) {
        self.compare_mode = !self.compare_mode;
        save_config(self);
    }

    /// Toggle list mode
    pub fn toggle_list_mode(&mut self) {
        self.list_mode = !self.list_mode;
        self.list_mode_override = true;
        self.update_view_state();
        save_config(self);
    }

    /// Disable list mode (Show Deck Anyway)
    pub fn show_deck_anyway(&mut self) {
        self.list_mode = false;
        self.list_mode_override = true;
        self.update_view_state();
        save_config(self);
    }

    /// Cycle focus region (Tab navigation)
    pub fn cycle_focus_region(&mut self, reverse: bool) {
        self.focus_region = match (self.focus_region, reverse) {
            (FocusRegion::ZoneField, false) => FocusRegion::CoreDeck,
            (FocusRegion::CoreDeck, false) => FocusRegion::CollapseControls,
            (FocusRegion::CollapseControls, false) => FocusRegion::ZoneField,
            (FocusRegion::ZoneField, true) => FocusRegion::CollapseControls,
            (FocusRegion::CoreDeck, true) => FocusRegion::ZoneField,
            (FocusRegion::CollapseControls, true) => FocusRegion::CoreDeck,
        };
    }

    /// Update display order based on current state
    fn update_display_order(&mut self) {
        self.display_order =
            compute_display_order(&self.selected_zones, self.dominant_zone, &self.favorites);
    }

    /// Check if we should auto-enable list mode (N > 8)
    fn check_list_mode_threshold(&mut self) {
        if !self.list_mode_override {
            self.list_mode = self.selected_zones.len() > 8;
        }
        self.update_view_state();
    }

    /// Update view state based on current settings
    fn update_view_state(&mut self) {
        self.view_state = if self.picker_state.is_open {
            ViewState::PickerOpen
        } else if self.list_mode {
            ViewState::ListView
        } else if self.focus_strength >= 0.8 {
            ViewState::CompositeView
        } else {
            ViewState::DeckView
        };
    }

    /// Update time data for all selected zones
    fn update_zone_times(&mut self) {
        self.zone_times.clear();
        for &tz in &self.selected_zones {
            self.zone_times.insert(tz, compute_time_data(tz));
        }
    }

    /// Get time data for the dominant zone
    pub fn dominant_time(&self) -> Option<&TimeData> {
        self.zone_times.get(&self.dominant_zone)
    }
}

fn save_config(model: &Model) {
    let config = Config {
        selected_zone_ids: model
            .selected_zones
            .iter()
            .map(|tz| tz.name().to_string())
            .collect(),
        dominant_zone_id: model.dominant_zone.name().to_string(),
        favorites: model
            .favorites
            .iter()
            .map(|tz| tz.name().to_string())
            .collect(),
        focus_strength: model.focus_strength,
        compare_mode: model.compare_mode,
        list_mode: model.list_mode,
        list_mode_override: model.list_mode_override,
        reduced_motion: model.reduced_motion,
    };
    if let Err(e) = shared::save_config(CLOCK_NAME, &config) {
        eprintln!("Failed to save config: {}", e);
    }
}

fn model(app: &App) -> Model {
    // Create window
    let window_id = app
        .new_window()
        .title("Chrono-Superposition")
        .size(1400, 800)
        .min_size(1100, 600)
        .view(view)
        .key_pressed(key_pressed)
        .mouse_pressed(mouse_pressed)
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

    // Parse timezones from config
    let selected_zones: Vec<Tz> = config
        .selected_zone_ids
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let selected_zones = if selected_zones.is_empty() {
        vec![DEFAULT_TZ.parse().unwrap()]
    } else {
        selected_zones
    };

    let dominant_zone: Tz = config
        .dominant_zone_id
        .parse()
        .unwrap_or_else(|_| selected_zones[0]);

    let favorites: Vec<Tz> = config
        .favorites
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    // Compute initial display order
    let display_order = compute_display_order(&selected_zones, dominant_zone, &favorites);

    // Compute initial time data
    let mut zone_times = HashMap::new();
    for &tz in &selected_zones {
        zone_times.insert(tz, compute_time_data(tz));
    }

    // Determine initial view state
    let list_mode_override = config.list_mode_override;
    let list_mode = if list_mode_override {
        config.list_mode
    } else {
        selected_zones.len() > 8
    };
    let view_state = if list_mode {
        ViewState::ListView
    } else if config.focus_strength >= 0.8 {
        ViewState::CompositeView
    } else {
        ViewState::DeckView
    };

    let window_rect = app.window_rect();

    Model {
        selected_zones,
        dominant_zone,
        favorites,
        zone_times,
        display_order,
        focus_strength: config.focus_strength,
        compare_mode: config.compare_mode,
        list_mode,
        list_mode_override,
        view_state,
        mouse_position: None,
        window_center: pt2(window_rect.x(), window_rect.y()),
        hovered_card_index: None,
        picker_state: PickerState::default(),
        reduced_motion: config.reduced_motion,
        animation_time: 0.0,
        focus_region: FocusRegion::default(),
        egui,
    }
}

fn update(app: &App, model: &mut Model, update: Update) {
    // Update animation time
    model.animation_time = update.since_start.as_secs_f32();

    // Update window center for parallax
    let window_rect = app.window_rect();
    model.window_center = pt2(window_rect.x(), window_rect.y());

    // Update time data for all zones
    model.update_zone_times();

    // Update view state
    model.update_view_state();

    // Collect state for UI (before borrowing egui)
    let selected_zones = model.selected_zones.clone();
    let dominant_zone = model.dominant_zone;
    let favorites = model.favorites.clone();
    let zone_times = model.zone_times.clone();
    let zone_count = model.selected_zones.len();
    let dominant_time_clone = model.dominant_time().cloned();
    let mut focus_strength = model.focus_strength;
    let mut compare_mode = model.compare_mode;
    let mut list_mode = model.list_mode;
    let mut reduced_motion = model.reduced_motion;

    // Begin egui frame
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();

    // Draw Zone Field (left panel)
    let zone_field_result: ZoneFieldResult = draw_zone_field(
        &ctx,
        &mut model.picker_state,
        &selected_zones,
        dominant_zone,
        &favorites,
        &zone_times,
    );

    // Draw Collapse Controls (right panel)
    let controls_result: CollapseControlsResult = draw_collapse_controls(
        &ctx,
        &mut focus_strength,
        &mut compare_mode,
        &mut list_mode,
        &mut reduced_motion,
        zone_count,
        dominant_time_clone.as_ref(),
    );

    drop(ctx);

    // Apply zone field results
    if let Some(tz) = zone_field_result.set_dominant {
        model.set_dominant(tz);
    }
    if let Some(tz) = zone_field_result.remove_zone {
        model.remove_zone(tz);
    }
    if let Some(tz) = zone_field_result.toggle_favorite {
        model.toggle_favorite(tz);
    }
    if let Some(tz) = zone_field_result.add_zone {
        model.add_zone(tz);
    }

    // Apply controls results
    if controls_result.focus_strength_changed {
        model.focus_strength = focus_strength;
        model.update_view_state();
        save_config(model);
    }
    if controls_result.compare_mode_changed {
        model.compare_mode = compare_mode;
        save_config(model);
    }
    if controls_result.list_mode_changed {
        model.list_mode = list_mode;
        model.list_mode_override = true;
        model.update_view_state();
        save_config(model);
    }
    if controls_result.reduced_motion_changed {
        model.reduced_motion = reduced_motion;
        save_config(model);
    }
    if controls_result.show_deck_anyway {
        model.show_deck_anyway();
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Clear background
    draw.background().color(colors::BACKGROUND);

    // Calculate core layout (center area between panels)
    let layout = CoreLayout::calculate(window_rect, LEFT_PANEL_WIDTH, RIGHT_PANEL_WIDTH);

    // Calculate card geometries
    let pointer_delta = model.mouse_position.map(|pos| {
        let dx = (pos.x - model.window_center.x) / (window_rect.w() / 2.0);
        let dy = (pos.y - model.window_center.y) / (window_rect.h() / 2.0);
        pt2(dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0))
    });

    let geometries: Vec<CardGeometry> = model
        .display_order
        .iter()
        .enumerate()
        .map(|(i, _)| {
            CardGeometry::compute(
                i,
                model.display_order.len(),
                model.focus_strength,
                pointer_delta,
                model.reduced_motion,
            )
        })
        .collect();

    // Draw based on view state
    match model.view_state {
        ViewState::DeckView | ViewState::PickerOpen => {
            // Show deck view when picker is open (picker overlays on top)
            draw_card_deck(
                &draw,
                &layout,
                &model.display_order,
                &model.zone_times,
                model.dominant_zone,
                &geometries,
                model.compare_mode,
                model.hovered_card_index,
                model.animation_time,
                model.reduced_motion,
            );
        }
        ViewState::CompositeView => {
            draw_composite_readout(
                &draw,
                &layout,
                &model.display_order,
                &model.zone_times,
                model.dominant_zone,
                model.compare_mode,
                model.animation_time,
            );
        }
        ViewState::ListView => {
            draw_list_view(
                &draw,
                &layout,
                &model.display_order,
                &model.zone_times,
                model.dominant_zone,
                model.compare_mode,
            );
        }
    }

    // Draw title (centered on window, not core area)
    draw.text("CHRONO-SUPERPOSITION")
        .x_y(0.0, window_rect.top() - 30.0)
        .color(colors::TEXT_PRIMARY)
        .font_size(18)
        .w(400.0);

    // Render to frame
    draw.to_frame(app, &frame).unwrap();

    // Render egui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    let mods = app.keys.mods;

    match key {
        // Escape - close picker or return to deck view
        Key::Escape => {
            if model.picker_state.is_open {
                model.picker_state.close();
                model.update_view_state();
            } else if model.list_mode {
                model.list_mode = false;
                model.list_mode_override = true;
                model.update_view_state();
                save_config(model);
            }
        }

        // Tab - cycle focus regions
        Key::Tab => {
            if !model.picker_state.is_open {
                model.cycle_focus_region(mods.shift());
            }
        }

        // Enter - set hovered card as dominant (when Core Deck is focused)
        Key::Return => {
            if !model.picker_state.is_open
                && model.focus_region == FocusRegion::CoreDeck
            {
                if let Some(idx) = model.hovered_card_index {
                    if idx < model.display_order.len() {
                        let tz = model.display_order[idx];
                        model.set_dominant(tz);
                    }
                }
            }
        }

        // Arrow keys - cycle dominance (when Core Deck is focused)
        Key::Up => {
            if !model.picker_state.is_open && model.focus_region == FocusRegion::CoreDeck {
                model.cycle_dominance(-1);
            }
        }
        Key::Down => {
            if !model.picker_state.is_open && model.focus_region == FocusRegion::CoreDeck {
                model.cycle_dominance(1);
            }
        }

        // C - toggle compare mode
        Key::C => {
            if !model.picker_state.is_open {
                model.toggle_compare_mode();
            }
        }

        // L - toggle list mode
        Key::L => {
            if !model.picker_state.is_open {
                model.toggle_list_mode();
            }
        }

        // F or / - focus search / open picker
        Key::F | Key::Slash => {
            if !model.picker_state.is_open {
                model.picker_state.open();
                model.update_view_state();
            } else {
                model.picker_state.should_focus_search = true;
            }
        }

        _ => {}
    }
}

fn mouse_pressed(_app: &App, model: &mut Model, button: MouseButton) {
    match button {
        MouseButton::Left => {
            if !model.picker_state.is_open {
                // If hovering over a card, set it as dominant
                if let Some(idx) = model.hovered_card_index {
                    if idx < model.display_order.len() {
                        let tz = model.display_order[idx];
                        model.set_dominant(tz);
                    }
                }
            }
        }
        MouseButton::Middle => {
            // Rotary input: middle click toggles compare mode
            if !model.picker_state.is_open {
                model.toggle_compare_mode();
            }
        }
        _ => {}
    }
}

fn mouse_wheel(_app: &App, model: &mut Model, delta: MouseScrollDelta, _phase: TouchPhase) {
    // Rotary input: scroll wheel cycles dominance
    if model.picker_state.is_open {
        return;
    }

    let scroll_y = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 50.0,
    };

    if scroll_y > 0.5 {
        model.cycle_dominance(-1); // Scroll up = previous
    } else if scroll_y < -0.5 {
        model.cycle_dominance(1); // Scroll down = next
    }
}

fn mouse_moved(app: &App, model: &mut Model, pos: Point2) {
    model.mouse_position = Some(pos);

    // Update hovered card index based on mouse position
    let window_rect = app.window_rect();
    let layout = CoreLayout::calculate(window_rect, LEFT_PANEL_WIDTH, RIGHT_PANEL_WIDTH);

    // Check if mouse is within core area
    if layout.contains(pos.x, pos.y) {
        // Simple hit testing - cards are stacked, so check from top (last) to bottom (first)
        let pointer_delta = model.mouse_position.map(|p| {
            let dx = (p.x - model.window_center.x) / (window_rect.w() / 2.0);
            let dy = (p.y - model.window_center.y) / (window_rect.h() / 2.0);
            pt2(dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0))
        });

        model.hovered_card_index = None;
        for i in (0..model.display_order.len()).rev() {
            let geom = CardGeometry::compute(
                i,
                model.display_order.len(),
                model.focus_strength,
                pointer_delta,
                model.reduced_motion,
            );
            let card_rect = geom.card_rect(&layout);
            if card_rect.contains(pos) {
                model.hovered_card_index = Some(i);
                break;
            }
        }
    } else {
        model.hovered_card_index = None;
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    model.egui.handle_raw_event(event);
}

