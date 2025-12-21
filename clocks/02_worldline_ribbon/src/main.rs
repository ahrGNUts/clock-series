//! Worldline Ribbon Clock
//!
//! A clock as a scrolling ribbon of time: the present is a cursor;
//! the ribbon moves beneath it. Users can scrub time to explore DST and offsets.

mod drawing;
mod ribbon;
mod ui;

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use shared::{compute_time_data, query_dst_transitions, DstTransition, TimeData, Validity};

use crate::drawing::{
    colors, draw_error_banner, draw_help_text, draw_ribbon, draw_time_display, draw_zoom_indicator,
    RibbonLayout,
};
use crate::ribbon::{
    format_cursor_time, RibbonViewport, Tick, DEFAULT_ZOOM_INDEX, ZOOM_LEVELS,
};
use crate::ui::{
    draw_dst_status, draw_scrub_controls, draw_toast, draw_timezone_bar, draw_timezone_picker,
    PickerState,
};

const CLOCK_NAME: &str = "worldline_ribbon";
const DEFAULT_TZ: &str = "America/Los_Angeles";

fn main() {
    nannou::app(model).update(update).run();
}

/// Application mode - Live or Scrub
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    /// Live mode - ribbon scrolls with current time
    Live,
    /// Scrub mode - user is exploring a different instant
    Scrub { ghost_instant: DateTime<Utc> },
}

impl Mode {
    fn is_scrub(&self) -> bool {
        matches!(self, Mode::Scrub { .. })
    }

    #[allow(dead_code)]
    fn ghost_instant(&self) -> Option<DateTime<Utc>> {
        match self {
            Mode::Scrub { ghost_instant } => Some(*ghost_instant),
            Mode::Live => None,
        }
    }
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_tz_id: String,
    favorites: Vec<String>,
    reduced_motion: bool,
    zoom_index: usize,
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
            zoom_index: DEFAULT_ZOOM_INDEX,
        }
    }
}

/// Drag state for scrubbing
#[derive(Debug, Clone, Default)]
struct DragState {
    is_dragging: bool,
    start_x: f32,
    start_instant: DateTime<Utc>,
}

/// Scroll axis lock state - prevents accidental axis switching mid-gesture
#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum ScrollLock {
    #[default]
    None,
    Horizontal,
    Vertical,
}

/// Scroll state for trackpad gestures
#[derive(Debug, Clone, Default)]
struct ScrollState {
    /// Which axis is currently locked
    lock: ScrollLock,
    /// Accumulated vertical scroll (for zoom)
    vertical_accumulator: f32,
    /// Accumulated horizontal scroll (for time scrub)
    horizontal_accumulator: f32,
}

/// Application state
struct Model {
    /// Current mode (Live or Scrub)
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
    /// Current zoom level index
    zoom_index: usize,
    /// Cached DST transitions
    dst_transitions: Vec<DstTransition>,
    /// Last center instant used for DST query (to avoid re-querying every frame)
    last_dst_query_instant: Option<DateTime<Utc>>,
    /// Drag state for mouse scrubbing
    drag_state: DragState,
    /// Scroll state for trackpad gestures (axis locking)
    scroll_state: ScrollState,
    /// Error message to display (if any)
    error_message: Option<String>,
    /// Toast message with display start time (auto-dismisses after timeout)
    toast: Option<(String, std::time::Instant)>,
    /// Last valid timezone (for reverting on invalid selection)
    last_valid_tz: Tz,
    /// Whether a DST transition is currently visible in the viewport
    transition_visible: bool,
    /// egui integration
    egui: Egui,
}

impl Model {
    fn seconds_per_pixel(&self) -> f32 {
        ZOOM_LEVELS[self.zoom_index]
    }

    fn center_instant(&self) -> DateTime<Utc> {
        match &self.mode {
            Mode::Live => Utc::now(),
            Mode::Scrub { ghost_instant } => *ghost_instant,
        }
    }

    fn enter_scrub(&mut self, instant: DateTime<Utc>) {
        self.mode = Mode::Scrub {
            ghost_instant: instant,
        };
    }

    fn return_to_live(&mut self) {
        self.mode = Mode::Live;
    }

    fn adjust_ghost(&mut self, delta_seconds: i64) {
        match &mut self.mode {
            Mode::Live => {
                // Enter scrub mode with current time adjusted
                let ghost = Utc::now() + Duration::seconds(delta_seconds);
                self.mode = Mode::Scrub {
                    ghost_instant: ghost,
                };
            }
            Mode::Scrub { ghost_instant } => {
                *ghost_instant = *ghost_instant + Duration::seconds(delta_seconds);
            }
        }
    }

    fn zoom_in(&mut self) {
        if self.zoom_index > 0 {
            self.zoom_index -= 1;
        }
    }

    fn zoom_out(&mut self) {
        if self.zoom_index < ZOOM_LEVELS.len() - 1 {
            self.zoom_index += 1;
        }
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
        zoom_index: model.zoom_index,
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
    // Create window with minimum size to prevent layout issues
    let window_id = app
        .new_window()
        .title("Worldline Ribbon")
        .size(1100, 600)
        .min_size(800, 500)
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

    // Validate zoom index
    let zoom_index = config.zoom_index.min(ZOOM_LEVELS.len() - 1);

    // Compute initial time data
    let time_data = compute_time_data(selected_tz);

    // Query initial DST transitions
    let now = Utc::now();
    let dst_transitions = query_dst_transitions(selected_tz, now, 7);

    Model {
        mode: Mode::Live,
        time_data,
        selected_tz,
        favorites,
        picker_state: PickerState::default(),
        reduced_motion: config.reduced_motion,
        zoom_index,
        dst_transitions,
        last_dst_query_instant: Some(now),
        drag_state: DragState::default(),
        scroll_state: ScrollState::default(),
        error_message: None,
        toast: None,
        last_valid_tz: selected_tz,
        transition_visible: false,
        egui,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    let center = model.center_instant();

    // Update time data
    model.time_data = shared::compute_time_data_at(model.selected_tz, center);

    // Check for validity issues
    if model.time_data.validity != Validity::Ok {
        model.error_message = Some(match model.time_data.validity {
            Validity::TzMissing => "Time zone data missing. Showing UTC.".to_string(),
            Validity::TzDataStale => "Time zone data may be outdated.".to_string(),
            Validity::Unknown => "Unknown time zone issue.".to_string(),
            Validity::Ok => unreachable!(),
        });
    }

    // Re-query DST transitions if center has moved significantly (more than 1 hour)
    let should_requery = match model.last_dst_query_instant {
        Some(last) => (center - last).num_hours().abs() > 1,
        None => true,
    };

    if should_requery {
        model.dst_transitions = query_dst_transitions(model.selected_tz, center, 7);
        model.last_dst_query_instant = Some(center);
    }

    // Check if any DST transition is visible in the current viewport
    // Viewport span is approximately window_width * seconds_per_pixel
    let viewport_half_span = Duration::hours(6); // Conservative estimate
    model.transition_visible = model.dst_transitions.iter().any(|t| {
        let delta = (t.instant_utc - center).num_seconds().abs();
        delta < viewport_half_span.num_seconds()
    });

    // Auto-dismiss toast after 3 seconds
    if let Some((_, start_time)) = &model.toast {
        if start_time.elapsed().as_secs_f32() > 3.0 {
            model.toast = None;
        }
    }

    // Begin egui frame
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();

    // Collect UI state
    let current_tz = model.selected_tz;
    let favorites_clone = model.favorites.clone();
    let time_data_clone = model.time_data.clone();
    let is_scrub = model.mode.is_scrub();
    let mut reduced_motion = model.reduced_motion;

    // Draw timezone bar (top)
    let bar_clicked = draw_timezone_bar(&ctx, &time_data_clone);
    if bar_clicked {
        model.picker_state.open();
    }

    // Draw timezone picker (if open)
    let picker_result = draw_timezone_picker(
        &ctx,
        &mut model.picker_state,
        current_tz,
        &favorites_clone,
    );

    // Draw scrub controls
    let scrub_result = draw_scrub_controls(
        &ctx,
        is_scrub,
        model.zoom_index,
        &mut reduced_motion,
    );

    // Show DST status card when a transition is visible in viewport
    if model.transition_visible {
        draw_dst_status(&ctx, &time_data_clone);
    }

    // Draw toast notification if active
    if let Some((ref message, start_time)) = model.toast {
        draw_toast(&ctx, message, start_time.elapsed().as_secs_f32());
    }

    // Now apply UI results
    drop(ctx);

    // Handle picker result
    if let Some(tz) = picker_result.selected_tz {
        model.selected_tz = tz;
        model.last_valid_tz = tz; // Track last valid selection
        model.time_data = compute_time_data(tz);
        model.error_message = None; // Clear any error on successful selection
        // Invalidate DST cache
        model.last_dst_query_instant = None;
        save_config(model);
    }
    if let Some(tz) = picker_result.toggle_favorite {
        toggle_favorite(&mut model.favorites, tz);
        save_config(model);
    }
    if picker_result.close_picker {
        model.picker_state.close();
    }

    // Handle scrub control results
    if scrub_result.return_to_now {
        model.return_to_live();
    }
    if scrub_result.zoom_in {
        model.zoom_in();
        save_config(model);
    }
    if scrub_result.zoom_out {
        model.zoom_out();
        save_config(model);
    }
    if let Some(delta) = scrub_result.step_time {
        model.adjust_ghost(delta);
    }
    if scrub_result.reduced_motion_changed {
        model.reduced_motion = reduced_motion;
        save_config(model);
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Clear background
    draw.background().color(colors::BACKGROUND);

    // Calculate layout
    let layout = RibbonLayout::calculate(window_rect);

    // Create viewport
    let viewport = RibbonViewport::new(
        model.center_instant(),
        model.seconds_per_pixel(),
        window_rect.w(),
        model.selected_tz,
    );

    // Generate ticks
    let ticks: Vec<Tick> = viewport.generate_ticks();

    // Draw the ribbon
    draw_ribbon(
        &draw,
        &viewport,
        &ticks,
        &model.dst_transitions,
        &layout,
        model.mode.is_scrub(),
        model.reduced_motion,
    );

    // Draw time display
    let time_text = format_cursor_time(model.center_instant(), model.selected_tz);
    let date_text = model.time_data.format_date();
    draw_time_display(
        &draw,
        &time_text,
        &date_text,
        &layout,
        model.mode.is_scrub(),
    );

    // Draw zoom indicator
    draw_zoom_indicator(&draw, model.seconds_per_pixel(), window_rect);

    // Draw help text
    draw_help_text(&draw, window_rect);

    // Draw error banner if needed
    if let Some(ref message) = model.error_message {
        draw_error_banner(&draw, message, window_rect);
    }

    // Render to frame
    draw.to_frame(app, &frame).unwrap();

    // Render egui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

fn key_pressed(app: &App, model: &mut Model, key: Key) {
    // Check for modifier keys
    let mods = app.keys.mods;

    match key {
        // Escape - close picker or return to live
        Key::Escape => {
            if model.picker_state.is_open {
                model.picker_state.close();
            } else if model.mode.is_scrub() {
                model.return_to_live();
            }
        }

        // Space - toggle Live/Scrub
        Key::Space => {
            if !model.picker_state.is_open {
                match &model.mode {
                    Mode::Live => {
                        model.enter_scrub(Utc::now());
                    }
                    Mode::Scrub { .. } => {
                        model.return_to_live();
                    }
                }
            }
        }

        // Slash - focus search
        Key::Slash => {
            if !model.picker_state.is_open {
                model.picker_state.open();
            } else {
                model.picker_state.should_focus_search = true;
            }
        }

        // Arrow keys - step time
        Key::Left => {
            if mods.ctrl() || mods.logo() {
                model.adjust_ghost(-3600); // -1 hour
            } else if mods.shift() {
                model.adjust_ghost(-60); // -1 minute
            } else {
                model.adjust_ghost(-1); // -1 second
            }
        }
        Key::Right => {
            if mods.ctrl() || mods.logo() {
                model.adjust_ghost(3600); // +1 hour
            } else if mods.shift() {
                model.adjust_ghost(60); // +1 minute
            } else {
                model.adjust_ghost(1); // +1 second
            }
        }

        // Zoom controls
        Key::Equals | Key::Plus => {
            if mods.ctrl() || mods.logo() {
                model.zoom_in();
                save_config(model);
            }
        }
        Key::Minus => {
            if mods.ctrl() || mods.logo() {
                model.zoom_out();
                save_config(model);
            }
        }

        // R - toggle reduced motion
        Key::R => {
            model.reduced_motion = !model.reduced_motion;
            save_config(model);
        }

        _ => {}
    }
}

fn mouse_pressed(app: &App, model: &mut Model, button: MouseButton) {
    if button == MouseButton::Left && !model.picker_state.is_open {
        let mouse_pos = app.mouse.position();
        let window_rect = app.window_rect();
        let layout = RibbonLayout::calculate(window_rect);

        // Check if mouse is within ribbon area
        let ribbon_top = layout.ribbon_center_y + layout.ribbon_height;
        let ribbon_bottom = layout.ribbon_center_y - layout.ribbon_height;

        if mouse_pos.y >= ribbon_bottom && mouse_pos.y <= ribbon_top {
            model.drag_state = DragState {
                is_dragging: true,
                start_x: mouse_pos.x,
                start_instant: model.center_instant(),
            };
        }
    }
}

fn mouse_released(_app: &App, model: &mut Model, button: MouseButton) {
    if button == MouseButton::Left {
        model.drag_state.is_dragging = false;
    }
}

fn mouse_moved(_app: &App, model: &mut Model, pos: Point2) {
    if model.drag_state.is_dragging {
        let delta_x = pos.x - model.drag_state.start_x;
        // Moving mouse right shows earlier time (ribbon scrolls left)
        let delta_seconds = (-delta_x * model.seconds_per_pixel()) as i64;
        let ghost_instant = model.drag_state.start_instant + Duration::seconds(delta_seconds);
        model.mode = Mode::Scrub { ghost_instant };
    }
}

fn mouse_wheel(_app: &App, model: &mut Model, delta: MouseScrollDelta, phase: TouchPhase) {
    // Vertical scroll = zoom, Horizontal scroll = time scrub
    // Uses axis locking to prevent accidental mode switching mid-gesture
    const LOCK_THRESHOLD: f32 = 8.0; // Pixels needed to commit to an axis
    const ZOOM_THRESHOLD: f32 = 30.0; // Accumulated pixels needed to trigger zoom

    // Reset scroll state when gesture ends
    if phase == TouchPhase::Ended || phase == TouchPhase::Cancelled {
        model.scroll_state = ScrollState::default();
        return;
    }

    match delta {
        MouseScrollDelta::LineDelta(x, y) => {
            // Discrete scroll (mouse wheel) - trigger immediately, no locking needed
            // Vertical = zoom
            if y > 0.0 {
                model.zoom_in();
                save_config(model);
            } else if y < 0.0 {
                model.zoom_out();
                save_config(model);
            }
            // Horizontal = time scrub (if mouse has horizontal scroll)
            if x != 0.0 {
                let seconds = (x * -10.0) as i64;
                model.adjust_ghost(seconds);
            }
        }
        MouseScrollDelta::PixelDelta(pos) => {
            let dx = pos.x as f32;
            let dy = pos.y as f32;

            // Accumulate in both directions
            model.scroll_state.horizontal_accumulator += dx;
            model.scroll_state.vertical_accumulator += dy;

            // If not yet locked, check if we should lock to an axis
            if model.scroll_state.lock == ScrollLock::None {
                let abs_h = model.scroll_state.horizontal_accumulator.abs();
                let abs_v = model.scroll_state.vertical_accumulator.abs();

                if abs_h >= LOCK_THRESHOLD && abs_h > abs_v * 1.5 {
                    // Lock to horizontal (time scrub)
                    model.scroll_state.lock = ScrollLock::Horizontal;
                } else if abs_v >= LOCK_THRESHOLD && abs_v > abs_h * 1.5 {
                    // Lock to vertical (zoom)
                    model.scroll_state.lock = ScrollLock::Vertical;
                }
            }

            // Apply the appropriate action based on lock state
            match model.scroll_state.lock {
                ScrollLock::None => {
                    // Not yet committed - don't do anything until we lock
                }
                ScrollLock::Vertical => {
                    // Zoom mode
                    if model.scroll_state.vertical_accumulator >= ZOOM_THRESHOLD {
                        model.zoom_in();
                        model.scroll_state.vertical_accumulator = 0.0;
                        save_config(model);
                    } else if model.scroll_state.vertical_accumulator <= -ZOOM_THRESHOLD {
                        model.zoom_out();
                        model.scroll_state.vertical_accumulator = 0.0;
                        save_config(model);
                    }
                }
                ScrollLock::Horizontal => {
                    // Time scrub mode - apply horizontal scroll directly
                    let delta_seconds =
                        (model.scroll_state.horizontal_accumulator * model.seconds_per_pixel() * 0.02) as i64;
                    if delta_seconds != 0 {
                        model.adjust_ghost(delta_seconds);
                        model.scroll_state.horizontal_accumulator = 0.0;
                    }
                }
            }
        }
    }
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    // Let egui handle raw events
    model.egui.handle_raw_event(event);

    // Resync time data when window regains focus (in case app was backgrounded)
    if let nannou::winit::event::WindowEvent::Focused(true) = event {
        // Invalidate DST cache to force refresh
        model.last_dst_query_instant = None;
        // Refresh time data immediately
        model.time_data = shared::compute_time_data_at(model.selected_tz, model.center_instant());
    }
}

