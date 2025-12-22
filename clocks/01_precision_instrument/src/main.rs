//! Precision Instrument Clock
//!
//! A clock as a calibrated instrument panel: crisp typography, grid-aligned readouts,
//! and a secondary "calibration ring" that visualizes seconds.

mod drawing;
mod ui;

use std::time::Instant;

use chrono_tz::Tz;
use nannou::prelude::*;
use nannou_egui::{self, Egui};
use serde::{Deserialize, Serialize};
use shared::{compute_time_data, TimeData, Validity};

use crate::drawing::{
    colors, draw_calibration_ring, draw_error_banner, draw_primary_readout, draw_toasts, Layout,
    ToastMessage,
};
use crate::ui::{
    draw_dst_status_card, draw_favorites_chips, draw_settings_panel, draw_timezone_bar,
    draw_timezone_picker, PickerState,
};

const CLOCK_NAME: &str = "precision_instrument";
const DEFAULT_TZ: &str = "America/Los_Angeles";
const TOAST_DURATION_SECS: f32 = 3.0;

fn main() {
    nannou::app(model).update(update).run();
}

/// Persisted configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    selected_tz_id: String,
    favorites: Vec<String>,
    reduced_motion: bool,
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
        }
    }
}

/// Application state
struct Model {
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
    /// Error message to display (if any)
    error_message: Option<String>,
    /// egui integration
    egui: Egui,
    /// Current mouse position
    mouse_pos: Point2,
    /// Active toast notifications
    toasts: Vec<ToastMessage>,
    /// Whether window is focused (for resync)
    is_focused: bool,
    /// Layout info for hover detection
    #[allow(dead_code)]
    ring_center: Point2,
    #[allow(dead_code)]
    ring_radius: f32,
}

fn save_config(model: &Model) {
    let config = Config {
        selected_tz_id: model.selected_tz.name().to_string(),
        favorites: model.favorites.iter().map(|tz| tz.name().to_string()).collect(),
        reduced_motion: model.reduced_motion,
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

fn add_toast(model: &mut Model, message: String) {
    model.toasts.push(ToastMessage {
        text: message,
        created_at: Instant::now(),
        duration_secs: TOAST_DURATION_SECS,
    });
}

fn model(app: &App) -> Model {
    // Disable default escape-to-close behavior - we handle Escape ourselves
    app.set_exit_on_escape(false);
    
    // Create window
    let window_id = app
        .new_window()
        .title("Precision Instrument Clock")
        .size(900, 600)
        .view(view)
        .key_pressed(key_pressed)
        .mouse_moved(mouse_moved)
        .focused(window_focused)
        .unfocused(window_unfocused)
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
    let time_data = compute_time_data(selected_tz);

    Model {
        time_data,
        selected_tz,
        favorites,
        picker_state: PickerState::default(),
        reduced_motion: config.reduced_motion,
        error_message: None,
        egui,
        mouse_pos: pt2(0.0, 0.0),
        toasts: Vec::new(),
        is_focused: true,
        ring_center: pt2(0.0, 0.0),
        ring_radius: 0.0,
    }
}

fn update(_app: &App, model: &mut Model, update: Update) {
    // Update time data every frame
    model.time_data = compute_time_data(model.selected_tz);

    // Check for validity issues
    if model.time_data.validity != Validity::Ok {
        model.error_message = Some(match model.time_data.validity {
            Validity::TzMissing => "Time zone data missing. Showing UTC.".to_string(),
            Validity::TzDataStale => "Time zone data may be outdated.".to_string(),
            Validity::Unknown => "Unknown time zone issue.".to_string(),
            Validity::Ok => unreachable!(),
        });
    }

    // Remove expired toasts
    model.toasts.retain(|toast| {
        toast.created_at.elapsed().as_secs_f32() < toast.duration_secs
    });

    // Begin egui frame
    model.egui.set_elapsed_time(update.since_start);
    let ctx = model.egui.begin_frame();

    // Collect UI state needed for drawing
    let current_tz = model.selected_tz;
    let favorites_clone = model.favorites.clone();
    let time_data_clone = model.time_data.clone();
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

    // Draw DST status card
    draw_dst_status_card(&ctx, &time_data_clone, current_tz);

    // Draw settings panel
    let settings_changed = draw_settings_panel(&ctx, &mut reduced_motion);

    // Draw favorites chips (bottom)
    let favorites_selection = draw_favorites_chips(&ctx, &favorites_clone, current_tz);

    // Now apply UI results after egui frame is done (ctx is dropped here)
    drop(ctx);

    // Handle picker result
    if let Some(tz) = picker_result.selected_tz {
        model.selected_tz = tz;
        model.time_data = compute_time_data(tz);
        model.error_message = None;
        save_config(model);
    }
    if let Some(tz) = picker_result.toggle_favorite {
        toggle_favorite(&mut model.favorites, tz);
        save_config(model);
    }
    if picker_result.close_picker {
        model.picker_state.close();
    }

    // Handle settings change
    if settings_changed {
        model.reduced_motion = reduced_motion;
        save_config(model);
    }

    // Handle favorites selection
    if let Some(tz) = favorites_selection {
        model.selected_tz = tz;
        model.time_data = compute_time_data(tz);
        model.error_message = None;
        save_config(model);
    }

    // Log accessible description periodically (for screen reader verification)
    // Only log once per second to avoid spam
    if model.time_data.second == 0 && model.time_data.second_fraction < 0.02 {
        println!("{}", model.time_data.accessible_description());
    }
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let window_rect = app.window_rect();

    // Clear background
    draw.background().color(colors::BACKGROUND);

    // Calculate layout
    let layout = Layout::calculate(window_rect);

    // Draw primary readout (left panel)
    draw_primary_readout(&draw, &model.time_data, layout.left_panel);

    // Draw calibration ring (right panel)
    let ring_radius = layout.right_panel.w().min(layout.right_panel.h()) * 0.4;
    let ring_center = layout.right_panel.xy();
    
    // Check if mouse is hovering over ring
    let mouse_dist = model.mouse_pos.distance(ring_center);
    let is_hovering_ring = mouse_dist < ring_radius * 1.1 && mouse_dist > ring_radius * 0.3;
    
    draw_calibration_ring(
        &draw,
        &model.time_data,
        ring_center,
        ring_radius,
        model.reduced_motion,
        is_hovering_ring,
    );

    // Draw error banner if needed
    if let Some(ref message) = model.error_message {
        draw_error_banner(&draw, message, window_rect);
    }

    // Draw toast notifications
    draw_toasts(&draw, &model.toasts, window_rect);

    // Render to frame
    draw.to_frame(app, &frame).unwrap();

    // Render egui on top
    model.egui.draw_to_frame(&frame).unwrap();
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    match key {
        // Escape closes picker (if open)
        Key::Escape => {
            if model.picker_state.is_open {
                model.picker_state.close();
            }
        }
        // Slash focuses search (opens picker if needed)
        Key::Slash => {
            if !model.picker_state.is_open {
                model.picker_state.open();
            } else {
                model.picker_state.should_focus_search = true;
            }
        }
        // Space/Enter opens picker when closed
        Key::Space | Key::Return => {
            if !model.picker_state.is_open {
                model.picker_state.open();
            }
        }
        // R toggles reduced motion
        Key::R => {
            model.reduced_motion = !model.reduced_motion;
            save_config(model);
            let msg = if model.reduced_motion {
                "Reduced motion enabled"
            } else {
                "Reduced motion disabled"
            };
            add_toast(model, msg.to_string());
        }
        // Arrow keys for picker navigation
        Key::Up => {
            if model.picker_state.is_open {
                model.picker_state.move_selection(-1);
            }
        }
        Key::Down => {
            if model.picker_state.is_open {
                model.picker_state.move_selection(1);
            }
        }
        _ => {}
    }
}

fn mouse_moved(_app: &App, model: &mut Model, pos: Point2) {
    model.mouse_pos = pos;
}

fn window_focused(_app: &App, model: &mut Model) {
    // Resync time immediately when window regains focus
    model.time_data = compute_time_data(model.selected_tz);
    model.is_focused = true;
}

fn window_unfocused(_app: &App, model: &mut Model) {
    model.is_focused = false;
}

fn raw_window_event(_app: &App, model: &mut Model, event: &nannou::winit::event::WindowEvent) {
    // Let egui handle raw events for keyboard and mouse input
    model.egui.handle_raw_event(event);
}
