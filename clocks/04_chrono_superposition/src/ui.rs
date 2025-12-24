//! UI module - egui panel components for the three-region layout
//!
//! Provides:
//! - Zone Field (left panel): search, zone toggles, favorites
//! - Collapse Controls (right panel): focus strength, compare mode, list mode
//! - Timezone picker overlay

use std::collections::HashMap;

use chrono::Utc;
use chrono_tz::Tz;
use nannou_egui::egui;
use shared::{search_timezones, DstChange, TimeData, Validity};

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
#[derive(Default)]
pub struct PickerResult {
    /// If Some, the user selected a timezone to add
    pub add_zone: Option<Tz>,
    /// If Some, toggle favorite status for this timezone
    pub toggle_favorite: Option<Tz>,
    /// If true, close the picker
    pub close_picker: bool,
}

/// Result of Zone Field panel interactions
#[derive(Default)]
pub struct ZoneFieldResult {
    /// Set a zone as dominant
    pub set_dominant: Option<Tz>,
    /// Remove a zone from selection
    pub remove_zone: Option<Tz>,
    /// Toggle favorite status
    pub toggle_favorite: Option<Tz>,
    /// Add a new zone
    pub add_zone: Option<Tz>,
}

/// Result of Collapse Controls panel interactions
#[derive(Default)]
pub struct CollapseControlsResult {
    /// Focus strength changed
    pub focus_strength_changed: bool,
    /// Compare mode toggled
    pub compare_mode_changed: bool,
    /// List mode toggled
    pub list_mode_changed: bool,
    /// Reduced motion toggled
    pub reduced_motion_changed: bool,
    /// Show Deck Anyway clicked
    pub show_deck_anyway: bool,
}

/// Draw the Zone Field panel (left side)
pub fn draw_zone_field(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    selected_zones: &[Tz],
    dominant_zone: Tz,
    favorites: &[Tz],
    zone_times: &HashMap<Tz, TimeData>,
) -> ZoneFieldResult {
    let mut result = ZoneFieldResult::default();

    egui::SidePanel::left("zone_field_panel")
        .resizable(false)
        .default_width(240.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Zone Field");
            ui.add_space(10.0);

            // Add Zone button
            if ui.button("+ Add Time Zone").clicked() {
                picker_state.open();
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Selected zones list
            ui.label(
                egui::RichText::new(format!("Selected Zones ({})", selected_zones.len()))
                    .size(12.0)
                    .color(egui::Color32::from_rgb(160, 165, 175)),
            );
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for &tz in selected_zones {
                        let is_dominant = tz == dominant_zone;
                        let is_favorite = favorites.contains(&tz);
                        let time_data = zone_times.get(&tz);

                        ui.horizontal(|ui| {
                            // Dominant indicator / click to set dominant
                            let dom_label = if is_dominant { "[*]" } else { "[ ]" };
                            if ui
                                .selectable_label(is_dominant, dom_label)
                                .on_hover_text("Click to set as dominant")
                                .clicked()
                            {
                                result.set_dominant = Some(tz);
                            }

                            // Favorite toggle
                            let fav_label = if is_favorite { "★" } else { "☆" };
                            if ui.small_button(fav_label).clicked() {
                                result.toggle_favorite = Some(tz);
                            }

                            // Zone info
                            ui.vertical(|ui| {
                                // Zone name (shortened)
                                let short_name: String = tz
                                    .name()
                                    .split('/')
                                    .last()
                                    .unwrap_or(tz.name())
                                    .chars()
                                    .take(18)
                                    .collect();

                                let name_color = if is_dominant {
                                    egui::Color32::from_rgb(245, 240, 235)
                                } else {
                                    egui::Color32::from_rgb(180, 185, 195)
                                };

                                ui.label(egui::RichText::new(&short_name).color(name_color));

                                // Time preview
                                if let Some(td) = time_data {
                                    let time_str = format!(
                                        "{}:{:02} {}",
                                        td.hour12, td.minute, td.meridiem
                                    );

                                    let mut time_label = egui::RichText::new(&time_str)
                                        .size(11.0)
                                        .color(egui::Color32::from_rgb(140, 145, 155));

                                    // DST warning indicator
                                    let has_dst_warning = matches!(
                                        td.dst_change,
                                        DstChange::Upcoming { .. } | DstChange::JustOccurred { .. }
                                    );
                                    if has_dst_warning {
                                        time_label =
                                            time_label.color(egui::Color32::from_rgb(255, 107, 53));
                                    }

                                    ui.label(time_label);
                                }
                            });

                            // Remove button (only if more than 1 zone)
                            if selected_zones.len() > 1 {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("×")
                                            .on_hover_text("Remove zone")
                                            .clicked()
                                        {
                                            result.remove_zone = Some(tz);
                                        }
                                    },
                                );
                            }
                        });

                        ui.add_space(4.0);
                    }
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Keyboard hints
            ui.label(
                egui::RichText::new("Keyboard:")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(140, 145, 155)),
            );
            ui.label(
                egui::RichText::new("Up/Down: Cycle dominant")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 125, 135)),
            );
            ui.label(
                egui::RichText::new("F  Search zones")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 125, 135)),
            );
        });

    // Draw timezone picker if open
    if picker_state.is_open {
        let picker_result = draw_timezone_picker(ctx, picker_state, selected_zones, favorites);
        if let Some(tz) = picker_result.add_zone {
            result.add_zone = Some(tz);
        }
        if let Some(tz) = picker_result.toggle_favorite {
            result.toggle_favorite = Some(tz);
        }
        if picker_result.close_picker {
            picker_state.close();
        }
    }

    result
}

/// Draw the Collapse Controls panel (right side)
pub fn draw_collapse_controls(
    ctx: &egui::Context,
    focus_strength: &mut f32,
    compare_mode: &mut bool,
    list_mode: &mut bool,
    reduced_motion: &mut bool,
    zone_count: usize,
    dominant_time: Option<&TimeData>,
) -> CollapseControlsResult {
    let mut result = CollapseControlsResult::default();

    egui::SidePanel::right("collapse_controls_panel")
        .resizable(false)
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Controls");
            ui.add_space(15.0);

            // Current dominant zone time display
            if let Some(td) = dominant_time {
                ui.label(
                    egui::RichText::new("Dominant Zone")
                        .size(11.0)
                        .color(egui::Color32::from_rgb(160, 165, 175)),
                );

                let time_str = format!(
                    "{}:{:02}:{:02} {}",
                    td.hour12, td.minute, td.second, td.meridiem
                );
                ui.label(
                    egui::RichText::new(&time_str)
                        .size(24.0)
                        .color(egui::Color32::from_rgb(245, 240, 235)),
                );

                ui.label(td.format_date());
                ui.label(
                    egui::RichText::new(&td.format_utc_offset())
                        .size(11.0)
                        .color(egui::Color32::from_rgb(140, 145, 155)),
                );

                // Validity warning
                if td.validity != Validity::Ok {
                    let warning = match td.validity {
                        Validity::TzMissing => "⚠ TZ data missing",
                        Validity::TzDataStale => "⚠ TZ data may be stale",
                        Validity::Unknown => "⚠ Unknown validity",
                        Validity::Ok => "",
                    };
                    ui.colored_label(egui::Color32::from_rgb(255, 179, 71), warning);
                }

                // DST status
                ui.add_space(5.0);
                draw_dst_status(ui, td);

                ui.add_space(15.0);
                ui.separator();
                ui.add_space(10.0);
            }

            // Focus Strength slider
            ui.label("Focus Strength");
            ui.add_space(3.0);

            let focus_label = if *focus_strength >= 0.8 {
                "Collapsed"
            } else if *focus_strength >= 0.4 {
                "Focused"
            } else {
                "Spread"
            };

            let slider_response = ui.add(
                egui::Slider::new(focus_strength, 0.0..=1.0)
                    .show_value(false)
                    .text(focus_label),
            );
            if slider_response.changed() {
                result.focus_strength_changed = true;
            }

            ui.label(
                egui::RichText::new("Low = spread cards, High = collapse")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 125, 135)),
            );

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Compare Mode toggle
            ui.label("Compare Mode");
            ui.add_space(3.0);

            if ui
                .checkbox(compare_mode, "Show deltas from dominant")
                .changed()
            {
                result.compare_mode_changed = true;
            }

            ui.label(
                egui::RichText::new("Keyboard: C")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 125, 135)),
            );

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // List Mode toggle
            ui.label("View Mode");
            ui.add_space(3.0);

            if ui.checkbox(list_mode, "List Mode (Accessible)").changed() {
                result.list_mode_changed = true;
            }

            if zone_count > 8 {
                if *list_mode {
                    // Show "Show Deck Anyway" button when list mode is auto-triggered
                    ui.add_space(5.0);
                    if ui.button("Show Deck Anyway").clicked() {
                        result.show_deck_anyway = true;
                    }
                    ui.label(
                        egui::RichText::new(format!("({} zones in deck)", zone_count))
                            .size(10.0)
                            .color(egui::Color32::from_rgb(120, 125, 135)),
                    );
                } else {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 179, 71),
                        format!("⚠ {} zones - list recommended", zone_count),
                    );
                }
            }

            ui.label(
                egui::RichText::new("Keyboard: L")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 125, 135)),
            );

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Settings
            ui.label("Settings");
            ui.add_space(3.0);

            if ui.checkbox(reduced_motion, "Reduced Motion").changed() {
                result.reduced_motion_changed = true;
            }

            ui.label(
                egui::RichText::new("Disables parallax and animations")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(120, 125, 135)),
            );

            ui.add_space(20.0);

            // Zone count
            ui.label(
                egui::RichText::new(format!("{} zones in superposition", zone_count))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(100, 105, 115)),
            );
        });

    result
}

/// Draw DST status section
fn draw_dst_status(ui: &mut egui::Ui, time_data: &TimeData) {
    let status_text = if time_data.is_dst {
        "Daylight Saving Time"
    } else {
        "Standard Time"
    };

    let status_color = if time_data.is_dst {
        egui::Color32::from_rgb(255, 179, 71)
    } else {
        egui::Color32::from_rgb(160, 165, 175)
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("DST:")
                .size(11.0)
                .color(egui::Color32::from_rgb(140, 145, 155)),
        );
        ui.label(egui::RichText::new(status_text).size(11.0).color(status_color));
    });

    // Transition warning with hours remaining
    let now = Utc::now();
    match &time_data.dst_change {
        DstChange::Upcoming {
            instant,
            delta_minutes,
        } => {
            let hours_remaining = (*instant - now).num_hours();
            let direction_sign = if *delta_minutes > 0 { "+" } else { "" };
            ui.colored_label(
                egui::Color32::from_rgb(255, 107, 53),
                format!(
                    "⚠ DST shift in {}h ({}{}m)",
                    hours_remaining,
                    direction_sign,
                    delta_minutes
                ),
            );
            ui.label(
                egui::RichText::new(format!("At {}", instant.format("%H:%M UTC")))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(140, 145, 155)),
            );
        }
        DstChange::JustOccurred {
            instant,
            delta_minutes,
        } => {
            let hours_ago = (now - *instant).num_hours();
            let direction_sign = if *delta_minutes > 0 { "+" } else { "" };
            ui.colored_label(
                egui::Color32::from_rgb(255, 179, 71),
                format!(
                    "DST shift {}h ago ({}{}m)",
                    hours_ago,
                    direction_sign,
                    delta_minutes
                ),
            );
            ui.label(
                egui::RichText::new(format!("At {}", instant.format("%H:%M UTC")))
                    .size(10.0)
                    .color(egui::Color32::from_rgb(140, 145, 155)),
            );
        }
        DstChange::None => {}
    }
}

/// Draw the timezone picker overlay
fn draw_timezone_picker(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    selected_zones: &[Tz],
    favorites: &[Tz],
) -> PickerResult {
    let mut result = PickerResult::default();

    egui::Window::new("Add Time Zone")
        .collapsible(false)
        .resizable(true)
        .default_width(450.0)
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
                    if !selected_zones.contains(&sys_tz) {
                        result.add_zone = Some(sys_tz);
                        result.close_picker = true;
                    }
                }
            }

            ui.separator();

            // Favorites section
            if !favorites.is_empty() {
                ui.label("Favorites:");
                ui.horizontal_wrapped(|ui| {
                    for &tz in favorites {
                        let already_selected = selected_zones.contains(&tz);
                        let label = if already_selected {
                            format!("★ {} ✓", short_zone_name(tz))
                        } else {
                            format!("★ {}", short_zone_name(tz))
                        };

                        let response = ui.selectable_label(already_selected, &label);
                        if response.clicked() && !already_selected {
                            result.add_zone = Some(tz);
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
                        let already_selected = selected_zones.contains(&tz);
                        let is_favorite = favorites.contains(&tz);

                        ui.horizontal(|ui| {
                            // Star button for favorites
                            let star = if is_favorite { "★" } else { "☆" };
                            if ui.small_button(star).clicked() {
                                result.toggle_favorite = Some(tz);
                            }

                            // Timezone name
                            let label = if already_selected {
                                format!("{} ✓", tz.name())
                            } else {
                                tz.name().to_string()
                            };

                            let response = ui.selectable_label(already_selected, &label);
                            if response.clicked() && !already_selected {
                                result.add_zone = Some(tz);
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

/// Get short zone name (last part after /)
fn short_zone_name(tz: Tz) -> String {
    tz.name()
        .split('/')
        .last()
        .unwrap_or(tz.name())
        .to_string()
}

