//! UI module - egui timezone picker, scrub controls, and settings
//!
//! Provides the interactive UI components using nannou_egui.

use chrono_tz::Tz;
use nannou_egui::egui;
use shared::{search_timezones, DstChange, TimeData};

use crate::ribbon::ZOOM_LEVELS;

/// State for the timezone picker
#[derive(Default)]
pub struct PickerState {
    /// Whether the picker is currently open
    pub is_open: bool,
    /// Current search query
    pub search_query: String,
    /// Cached search results
    pub search_results: Vec<Tz>,
    /// Whether the search field should be focused
    pub should_focus_search: bool,
}

impl PickerState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.search_query.clear();
        self.search_results = search_timezones("");
        self.should_focus_search = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.search_query.clear();
        self.search_results.clear();
    }

    pub fn update_search(&mut self) {
        self.search_results = search_timezones(&self.search_query);
    }
}

/// Result of timezone picker interactions
pub struct PickerResult {
    /// If Some, the user selected a new timezone
    pub selected_tz: Option<Tz>,
    /// If Some, toggle favorite status for this timezone
    pub toggle_favorite: Option<Tz>,
    /// If true, close the picker
    pub close_picker: bool,
}

impl Default for PickerResult {
    fn default() -> Self {
        Self {
            selected_tz: None,
            toggle_favorite: None,
            close_picker: false,
        }
    }
}

/// Result of scrub control interactions
pub struct ScrubControlResult {
    /// Return to live mode
    pub return_to_now: bool,
    /// Step time by this many seconds
    pub step_time: Option<i64>,
    /// Zoom in
    pub zoom_in: bool,
    /// Zoom out
    pub zoom_out: bool,
    /// Reduced motion setting changed
    pub reduced_motion_changed: bool,
}

impl Default for ScrubControlResult {
    fn default() -> Self {
        Self {
            return_to_now: false,
            step_time: None,
            zoom_in: false,
            zoom_out: false,
            reduced_motion_changed: false,
        }
    }
}

/// Draw the timezone picker overlay
pub fn draw_timezone_picker(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    current_tz: Tz,
    favorites: &[Tz],
) -> PickerResult {
    let mut result = PickerResult::default();

    if !picker_state.is_open {
        return result;
    }

    egui::Window::new("Select Time Zone")
        .collapsible(false)
        .resizable(true)
        .default_width(400.0)
        .default_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Search field
            ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut picker_state.search_query);
                if picker_state.should_focus_search {
                    response.request_focus();
                    picker_state.should_focus_search = false;
                }
                if response.changed() {
                    picker_state.update_search();
                }
            });

            ui.separator();

            // System timezone button
            if ui.button("Use System Time Zone").clicked() {
                if let Some(sys_tz) = shared::system_timezone() {
                    result.selected_tz = Some(sys_tz);
                    result.close_picker = true;
                }
            }

            ui.separator();

            // Favorites section
            if !favorites.is_empty() {
                ui.label("Favorites:");
                ui.horizontal_wrapped(|ui| {
                    for &tz in favorites {
                        let is_current = tz == current_tz;
                        let label = if is_current {
                            format!("★ {} ◀", tz.name())
                        } else {
                            format!("★ {}", tz.name())
                        };
                        if ui.selectable_label(is_current, &label).clicked() {
                            result.selected_tz = Some(tz);
                            result.close_picker = true;
                        }
                    }
                });
                ui.separator();
            }

            // Results list
            ui.label(format!(
                "{} time zones found",
                picker_state.search_results.len()
            ));

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for &tz in &picker_state.search_results {
                        let is_current = tz == current_tz;
                        let is_favorite = favorites.contains(&tz);

                        ui.horizontal(|ui| {
                            // Star button for favorites
                            let star = if is_favorite { "★" } else { "☆" };
                            if ui.small_button(star).clicked() {
                                result.toggle_favorite = Some(tz);
                            }

                            // Timezone name
                            let label = if is_current {
                                format!("{} ◀", tz.name())
                            } else {
                                tz.name().to_string()
                            };
                            if ui.selectable_label(is_current, &label).clicked() {
                                result.selected_tz = Some(tz);
                                result.close_picker = true;
                            }
                        });
                    }
                });

            ui.separator();

            // Close button
            if ui.button("Close").clicked() {
                result.close_picker = true;
            }
        });

    // Handle escape key
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        result.close_picker = true;
    }

    result
}

/// Draw the scrub controls panel
pub fn draw_scrub_controls(
    ctx: &egui::Context,
    is_scrub_mode: bool,
    current_zoom_index: usize,
    reduced_motion: &mut bool,
) -> ScrubControlResult {
    let mut result = ScrubControlResult::default();

    egui::Window::new("Controls")
        .collapsible(true)
        .resizable(false)
        .default_width(200.0)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .show(ctx, |ui| {
            // Return to Now button (only in scrub mode)
            if is_scrub_mode {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("⏱ Return to Now")
                                .color(egui::Color32::from_rgb(40, 30, 20)),
                        )
                        .fill(egui::Color32::from_rgb(255, 179, 71)),
                    )
                    .clicked()
                {
                    result.return_to_now = true;
                }
                ui.separator();
            }

            // Time step controls
            ui.label("Step Time:");
            ui.horizontal(|ui| {
                if ui.button("−1h").clicked() {
                    result.step_time = Some(-3600);
                }
                if ui.button("−1m").clicked() {
                    result.step_time = Some(-60);
                }
                if ui.button("+1m").clicked() {
                    result.step_time = Some(60);
                }
                if ui.button("+1h").clicked() {
                    result.step_time = Some(3600);
                }
            });

            ui.separator();

            // Zoom controls
            ui.label("Zoom:");
            ui.horizontal(|ui| {
                let can_zoom_in = current_zoom_index > 0;
                let can_zoom_out = current_zoom_index < ZOOM_LEVELS.len() - 1;

                if ui.add_enabled(can_zoom_in, egui::Button::new("🔍+")).clicked() {
                    result.zoom_in = true;
                }
                ui.label(format!("{:.0} sec/px", ZOOM_LEVELS[current_zoom_index]));
                if ui.add_enabled(can_zoom_out, egui::Button::new("🔍−")).clicked() {
                    result.zoom_out = true;
                }
            });

            ui.separator();

            // Reduced motion toggle
            if ui.checkbox(reduced_motion, "Reduced Motion").changed() {
                result.reduced_motion_changed = true;
            }
            ui.label("Disables warp effect");
        });

    result
}

/// Draw the DST status panel (shown when DST transition is in viewport)
pub fn draw_dst_status(ctx: &egui::Context, time_data: &TimeData) {
    egui::Window::new("DST Status")
        .collapsible(true)
        .resizable(false)
        .default_width(250.0)
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 50.0])
        .show(ctx, |ui| {
            // Current DST status
            ui.horizontal(|ui| {
                ui.label("Status:");
                if time_data.is_dst {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 179, 71),
                        "Daylight Saving Time",
                    );
                } else {
                    ui.label("Standard Time");
                }
            });

            ui.separator();

            // DST transition info
            match &time_data.dst_change {
                DstChange::None => {
                    ui.label("No DST transitions within 24 hours.");
                }
                DstChange::Upcoming {
                    instant,
                    delta_minutes,
                } => {
                    let direction = if *delta_minutes > 0 {
                        "forward"
                    } else {
                        "back"
                    };
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 107, 53),
                        "⚠ Upcoming DST Change",
                    );
                    ui.label(format!(
                        "Clocks will move {} by {} minutes",
                        direction,
                        delta_minutes.abs()
                    ));
                    ui.label(format!("At: {}", instant.format("%Y-%m-%d %H:%M UTC")));
                }
                DstChange::JustOccurred {
                    instant,
                    delta_minutes,
                } => {
                    let direction = if *delta_minutes > 0 {
                        "forward"
                    } else {
                        "back"
                    };
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 179, 71),
                        "ℹ Recent DST Change",
                    );
                    ui.label(format!(
                        "Clocks moved {} by {} minutes",
                        direction,
                        delta_minutes.abs()
                    ));
                    ui.label(format!("At: {}", instant.format("%Y-%m-%d %H:%M UTC")));
                }
            }
        });
}

/// Draw the main timezone info bar (clickable to open picker)
pub fn draw_timezone_bar(ctx: &egui::Context, time_data: &TimeData) -> bool {
    let mut clicked = false;

    egui::TopBottomPanel::top("tz_bar")
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let tz_text = format!(
                    "{} ({}) · {} · DST {}",
                    time_data.local_datetime.timezone().name(),
                    time_data.tz_abbrev,
                    time_data.format_utc_offset(),
                    if time_data.is_dst { "On" } else { "Off" }
                );

                if ui
                    .add(egui::Label::new(&tz_text).sense(egui::Sense::click()))
                    .on_hover_text("Click to change time zone")
                    .clicked()
                {
                    clicked = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // DST indicator
                    match &time_data.dst_change {
                        DstChange::None => {}
                        DstChange::Upcoming { delta_minutes, .. } => {
                            let sign = if *delta_minutes > 0 { "+" } else { "" };
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 107, 53),
                                format!("⚠ DST {}{}m soon", sign, delta_minutes),
                            );
                        }
                        DstChange::JustOccurred { delta_minutes, .. } => {
                            let sign = if *delta_minutes > 0 { "+" } else { "" };
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 179, 71),
                                format!("DST {}{}m occurred", sign, delta_minutes),
                            );
                        }
                    }
                });
            });
        });

    clicked
}

/// Draw a toast notification that auto-dismisses
pub fn draw_toast(ctx: &egui::Context, message: &str, elapsed_secs: f32) {
    // Fade out during last 0.5 seconds
    let alpha = if elapsed_secs > 2.5 {
        1.0 - (elapsed_secs - 2.5) * 2.0
    } else {
        1.0
    };

    if alpha <= 0.0 {
        return;
    }

    let alpha_u8 = (alpha * 255.0) as u8;

    egui::Area::new("toast")
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -80.0])
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgba_unmultiplied(60, 40, 30, alpha_u8))
                .rounding(8.0)
                .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(message)
                            .color(egui::Color32::from_rgba_unmultiplied(255, 200, 150, alpha_u8))
                            .size(14.0),
                    );
                });
        });
}

