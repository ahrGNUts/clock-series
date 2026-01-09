//! UI module for the Ritual Clock
//!
//! Provides the conductor panel with timezone picker, DST indicator,
//! and gesture sensitivity controls using egui.

use chrono_tz::Tz;
use nannou_egui::egui;
use shared::{search_timezones, system_timezone, DstChange, TimeData};

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

/// Result of conductor panel interactions
#[derive(Default)]
pub struct ConductorPanelResult {
    /// Set a new timezone
    pub set_timezone: Option<Tz>,
    /// Toggle favorite status
    pub toggle_favorite: Option<Tz>,
    /// Sensitivity slider changed
    pub sensitivity_changed: bool,
    /// Overlay toggle changed
    pub overlay_changed: bool,
    /// Reduced motion changed
    pub reduced_motion_changed: bool,
}

/// Draw the conductor panel (bottom)
pub fn draw_conductor_panel(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    selected_zone: Tz,
    favorites: &[Tz],
    time_data: &TimeData,
    gesture_sensitivity: &mut f32,
    overlay_always_on: &mut bool,
    reduced_motion: &mut bool,
    trails_enabled_in_reduced_motion: &mut bool,
) -> ConductorPanelResult {
    let mut result = ConductorPanelResult::default();

    egui::TopBottomPanel::bottom("conductor_panel")
        .resizable(false)
        .min_height(100.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                // Left section: Timezone info and picker
                ui.vertical(|ui| {
                    ui.heading("Ensemble");

                    // Current timezone display
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format_zone_name(selected_zone))
                                .size(14.0),
                        );
                        ui.label(
                            egui::RichText::new(format!("({})", time_data.tz_abbrev))
                                .size(12.0)
                                .color(egui::Color32::from_rgb(140, 150, 170)),
                        );
                    });

                    // Open picker button
                    if ui.button("Change Ensemble (T)")
                        .on_hover_text("Open timezone picker. Keyboard: T")
                        .clicked()
                    {
                        picker_state.open();
                    }

                    // System timezone shortcut
                    if ui.button("Use System Time")
                        .on_hover_text("Set to your computer's local timezone")
                        .clicked()
                    {
                        if let Some(sys_tz) = system_timezone() {
                            result.set_timezone = Some(sys_tz);
                        }
                    }
                });

                ui.separator();

                // Center section: DST indicator
                ui.vertical(|ui| {
                    ui.heading("DST Status");

                    // DST active indicator
                    let dst_text = if time_data.is_dst {
                        egui::RichText::new("● Daylight Saving Time Active")
                            .color(egui::Color32::from_rgb(255, 179, 71))
                    } else {
                        egui::RichText::new("○ Standard Time")
                            .color(egui::Color32::from_rgb(140, 150, 170))
                    };
                    ui.label(dst_text);

                    // DST warning
                    match &time_data.dst_change {
                        DstChange::Upcoming { instant, delta_minutes } => {
                            let hours_until = (*instant - chrono::Utc::now())
                                .num_hours();
                            let direction = if *delta_minutes > 0 {
                                "spring forward"
                            } else {
                                "fall back"
                            };
                            ui.label(
                                egui::RichText::new(format!(
                                    "⚠ DST change in {}h ({})",
                                    hours_until, direction
                                ))
                                .color(egui::Color32::from_rgb(255, 150, 80)),
                            );
                        }
                        DstChange::JustOccurred { delta_minutes, .. } => {
                            let direction = if *delta_minutes > 0 {
                                "sprang forward"
                            } else {
                                "fell back"
                            };
                            ui.label(
                                egui::RichText::new(format!("ℹ Clocks {} recently", direction))
                                    .color(egui::Color32::from_rgb(100, 180, 255)),
                            );
                        }
                        DstChange::None => {}
                    }
                });

                ui.separator();

                // Right section: Sensitivity and settings
                ui.vertical(|ui| {
                    ui.heading("Controls");

                    // Gesture sensitivity slider
                    ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        let old_sensitivity = *gesture_sensitivity;
                        let slider = egui::Slider::new(gesture_sensitivity, 0.0..=1.0)
                            .show_value(false)
                            .text("Gesture trail sensitivity");
                        ui.add(slider)
                            .on_hover_text("Adjust the intensity of gesture trails");
                        if (*gesture_sensitivity - old_sensitivity).abs() > 0.001 {
                            result.sensitivity_changed = true;
                        }
                    });

                    // Overlay always-on toggle
                    let overlay_response = ui.checkbox(overlay_always_on, "Always show time (S)")
                        .on_hover_text("Keep digital time display visible. Keyboard: S");
                    if overlay_response.changed() {
                        result.overlay_changed = true;
                    }

                    // Reduced motion toggle
                    let reduced_response = ui.checkbox(reduced_motion, "Reduced motion")
                        .on_hover_text("Disable continuous animations for accessibility");
                    if reduced_response.changed() {
                        result.reduced_motion_changed = true;
                    }

                    // Trails toggle (only visible in reduced motion)
                    if *reduced_motion {
                        let _ = ui.checkbox(trails_enabled_in_reduced_motion, "Enable trails anyway")
                            .on_hover_text("Allow gesture trails even in reduced motion mode");
                    }
                });
            });
        });

    // Draw picker overlay if open
    if picker_state.is_open {
        let picker_result = draw_timezone_picker(ctx, picker_state, favorites);

        if let Some(tz) = picker_result.select_zone {
            result.set_timezone = Some(tz);
            picker_state.close();
        }
        if let Some(tz) = picker_result.toggle_favorite {
            result.toggle_favorite = Some(tz);
        }
        if picker_result.close {
            picker_state.close();
        }
    }

    result
}

/// Result of timezone picker interactions
#[derive(Default)]
struct PickerResult {
    select_zone: Option<Tz>,
    toggle_favorite: Option<Tz>,
    close: bool,
}

/// Draw the timezone picker overlay
fn draw_timezone_picker(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    favorites: &[Tz],
) -> PickerResult {
    let mut result = PickerResult::default();

    egui::Window::new("Select Ensemble")
        .collapsible(false)
        .resizable(true)
        .default_width(400.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            // Search field
            ui.horizontal(|ui| {
                ui.label("Search:");
                let search_response = ui.text_edit_singleline(&mut picker_state.search_query);

                if picker_state.should_focus_search {
                    search_response.request_focus();
                    picker_state.should_focus_search = false;
                }

                if search_response.changed() {
                    picker_state.update_search();
                }
            });

            ui.separator();

            // Favorites section
            if !favorites.is_empty() {
                ui.collapsing("★ Favorites", |ui| {
                    for &tz in favorites {
                        ui.horizontal(|ui| {
                            if ui.button(format_zone_name(tz)).clicked() {
                                result.select_zone = Some(tz);
                            }
                            if ui.small_button("★").clicked() {
                                result.toggle_favorite = Some(tz);
                            }
                        });
                    }
                });
                ui.separator();
            }

            // System timezone shortcut
            if ui.button("📍 Use System Timezone").clicked() {
                if let Some(sys_tz) = system_timezone() {
                    result.select_zone = Some(sys_tz);
                }
            }

            ui.separator();

            // Search results
            ui.label(format!("{} results", picker_state.search_results.len()));

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for &tz in picker_state.search_results.iter().take(100) {
                        let is_favorite = favorites.contains(&tz);
                        ui.horizontal(|ui| {
                            if ui.button(format_zone_name(tz)).clicked() {
                                result.select_zone = Some(tz);
                            }
                            let fav_label = if is_favorite { "★" } else { "☆" };
                            if ui.small_button(fav_label).clicked() {
                                result.toggle_favorite = Some(tz);
                            }
                        });
                    }
                });

            ui.separator();

            if ui.button("Close (Esc)").clicked() {
                result.close = true;
            }
        });

    result
}

/// Format timezone name for display
fn format_zone_name(tz: Tz) -> String {
    let name = tz.name();
    // Extract city name from "Continent/City" format
    if let Some(idx) = name.rfind('/') {
        name[idx + 1..].replace('_', " ")
    } else {
        name.to_string()
    }
}

