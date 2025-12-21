//! UI module - egui side panel components
//!
//! Provides the interactive UI components using nannou_egui:
//! - SidePanel with time readout, timezone picker, DST status, legend
//! - Timezone picker overlay
//! - Inspect mode controls

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
    /// If Some, the user selected a new timezone
    pub selected_tz: Option<Tz>,
    /// If Some, toggle favorite status for this timezone
    pub toggle_favorite: Option<Tz>,
    /// If true, close the picker
    pub close_picker: bool,
}

/// Result of side panel interactions
#[derive(Default)]
pub struct SidePanelResult {
    /// Open the timezone picker
    pub open_picker: bool,
    /// Return to live mode
    pub return_to_now: bool,
    /// Reduced motion setting changed
    pub reduced_motion_changed: bool,
    /// Legend visibility changed
    pub legend_toggled: bool,
}

/// Result of inspect tooltip interactions
#[derive(Default)]
#[allow(dead_code)]
pub struct InspectResult {
    /// Pin/unpin the current inspection point
    pub toggle_pin: bool,
    /// Return to live mode
    pub return_to_now: bool,
}

/// Draw the main side panel with all UI elements
pub fn draw_side_panel(
    ctx: &egui::Context,
    time_data: &TimeData,
    is_inspecting: bool,
    inspect_time_str: Option<&str>,
    inspect_is_gap: bool,
    inspect_is_overlap: bool,
    reduced_motion: &mut bool,
    show_legend: &mut bool,
) -> SidePanelResult {
    let mut result = SidePanelResult::default();

    egui::SidePanel::right("side_panel")
        .resizable(false)
        .default_width(280.0)
        .show(ctx, |ui| {
            // === ERROR BANNER (if timezone data is invalid) ===
            if time_data.validity != Validity::Ok {
                draw_error_banner(ui, &time_data.validity);
                ui.add_space(5.0);
            }
            
            // === STICKY TIME SECTION (outside ScrollArea) ===
            ui.add_space(10.0);

            // Explicit Time Readout
            ui.heading("Current Time");
            ui.add_space(5.0);

            let time_str = if let Some(inspect) = inspect_time_str {
                inspect.to_string()
            } else {
                time_data.format_time()
            };

            ui.label(
                egui::RichText::new(&time_str)
                    .size(32.0)
                    .color(if is_inspecting {
                        egui::Color32::from_rgb(120, 180, 220)
                    } else {
                        egui::Color32::from_rgb(245, 230, 211)
                    }),
            );

            ui.label(
                egui::RichText::new(time_data.meridiem.to_string())
                    .size(18.0)
                    .color(egui::Color32::from_rgb(166, 144, 128)),
            );

            ui.add_space(5.0);
            ui.label(time_data.format_date());

            if is_inspecting {
                ui.add_space(5.0);
                ui.colored_label(
                    egui::Color32::from_rgb(120, 180, 220),
                    "◆ INSPECT MODE",
                );
                
                // Show DST gap/overlap warnings
                if inspect_is_gap {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 107, 53),
                        "⚠ Nonexistent time (DST gap)",
                    );
                } else if inspect_is_overlap {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 179, 71),
                        "⚠ Ambiguous time (DST overlap)",
                    );
                }
                
                if ui.button("Return to Now").clicked() {
                    result.return_to_now = true;
                }
            }

            ui.add_space(15.0);
            ui.separator();
            
            // === SCROLLABLE SECTION ===
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
            ui.add_space(10.0);

            // Timezone section
            ui.heading("Time Zone");
            ui.add_space(5.0);

            let tz_text = format!(
                "{}\n({}) {}",
                time_data.local_datetime.timezone().name(),
                time_data.tz_abbrev,
                time_data.format_utc_offset(),
            );

            if ui
                .add(egui::Label::new(&tz_text).sense(egui::Sense::click()))
                .on_hover_text("Click to change time zone")
                .clicked()
            {
                result.open_picker = true;
            }

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // DST Status Card
            ui.heading("DST Status");
            ui.add_space(5.0);

            draw_dst_status_card(ui, time_data);

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Legend section
            ui.horizontal(|ui| {
                ui.heading("Legend");
                if ui.small_button(if *show_legend { "▼" } else { "▶" }).clicked() {
                    *show_legend = !*show_legend;
                    result.legend_toggled = true;
                }
            });

            if *show_legend {
                ui.add_space(5.0);
                draw_legend(ui);
            }

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Accessibility - Map Summary
            ui.heading("Map Summary");
            ui.add_space(5.0);
            
            let summary = generate_map_summary(time_data, is_inspecting, inspect_time_str);
            ui.label(
                egui::RichText::new(&summary)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(180, 175, 170)),
            );

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            // Settings
            ui.heading("Settings");
            ui.add_space(5.0);

            if ui.checkbox(reduced_motion, "Reduced Motion").changed() {
                result.reduced_motion_changed = true;
            }
            ui.label(
                egui::RichText::new("Disables beacon pulse animation")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(140, 130, 120)),
            );
            
            ui.add_space(10.0);
                }); // End ScrollArea
        }); // End SidePanel

    result
}

/// Draw the DST status card
fn draw_dst_status_card(ui: &mut egui::Ui, time_data: &TimeData) {
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

    // DST transition info
    match &time_data.dst_change {
        DstChange::None => {
            ui.label(
                egui::RichText::new("No transitions within 24 hours")
                    .size(11.0)
                    .color(egui::Color32::from_rgb(140, 130, 120)),
            );
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
                "Clocks will move {} by {} min",
                direction,
                delta_minutes.abs()
            ));
            ui.label(
                egui::RichText::new(format!("At: {}", instant.format("%H:%M UTC")))
                    .size(11.0),
            );
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
                "Clocks moved {} by {} min",
                direction,
                delta_minutes.abs()
            ));
            ui.label(
                egui::RichText::new(format!("At: {}", instant.format("%H:%M UTC")))
                    .size(11.0),
            );
        }
    }
}

/// Generate an accessible map summary description
fn generate_map_summary(time_data: &TimeData, is_inspecting: bool, inspect_time_str: Option<&str>) -> String {
    let mode = if is_inspecting { "Inspecting" } else { "Live" };
    let default_time = time_data.format_time();
    let time_str = inspect_time_str.unwrap_or(&default_time);
    
    // Describe the terrain position
    let hour = time_data.hour24;
    let progress_in_hour = time_data.minute as f32 / 60.0;
    
    let terrain_desc = if progress_in_hour < 0.25 {
        "descending into valley"
    } else if progress_in_hour < 0.5 {
        "at valley floor"
    } else if progress_in_hour < 0.75 {
        "ascending toward peak"
    } else {
        "approaching peak"
    };
    
    let day_progress = (hour as f32 + time_data.minute as f32 / 60.0) / 24.0 * 100.0;
    
    let dst_status = if time_data.is_dst {
        "Daylight Saving Time active"
    } else {
        "Standard Time"
    };
    
    format!(
        "{} mode. Current position: {} {}. \
         The beacon marks {} on the day map, {:.0}% through the day. \
         Terrain is {}. {}.",
        mode,
        time_str,
        time_data.meridiem,
        time_str,
        day_progress,
        terrain_desc,
        dst_status
    )
}

/// Draw an error banner for invalid timezone data
fn draw_error_banner(ui: &mut egui::Ui, validity: &Validity) {
    let (message, color) = match validity {
        Validity::Ok => return,
        Validity::TzMissing => (
            "⚠ Time zone data missing. Showing UTC.",
            egui::Color32::from_rgb(255, 107, 53),
        ),
        Validity::TzDataStale => (
            "⚠ Time zone data may be outdated.",
            egui::Color32::from_rgb(255, 179, 71),
        ),
        Validity::Unknown => (
            "⚠ Time zone validity unknown.",
            egui::Color32::from_rgb(180, 180, 180),
        ),
    };
    
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(60, 40, 40))
        .inner_margin(8.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.colored_label(color, message);
        });
}

/// Draw the map legend
fn draw_legend(ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        // Terrain explanation
        ui.label(
            egui::RichText::new("Reading the Map:")
                .size(12.0)
                .color(egui::Color32::from_rgb(200, 190, 180)),
        );
        ui.add_space(3.0);
        
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(139, 119, 101), "●");
            ui.label("Peaks = late in hour");
        });
        
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(70, 100, 90), "●");
            ui.label("Valleys = early in hour");
        });
        
        ui.add_space(5.0);
        
        // Beacon
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(255, 179, 71), "◆");
            ui.label("Locator Beacon (now)");
        });
        
        // Grid lines
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(100, 100, 100), "│");
            ui.label("Hour boundaries");
        });
        
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(60, 60, 60), "┆");
            ui.label("15-minute marks");
        });
        
        ui.add_space(5.0);
        
        // DST markers
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(255, 107, 53), "║");
            ui.label("DST fault line");
        });
        
        ui.add_space(5.0);
        
        // Interaction hints
        ui.label(
            egui::RichText::new("Interactions:")
                .size(12.0)
                .color(egui::Color32::from_rgb(200, 190, 180)),
        );
        ui.add_space(3.0);
        
        ui.label(
            egui::RichText::new("Click map to inspect time")
                .size(11.0)
                .color(egui::Color32::from_rgb(140, 130, 120)),
        );
        ui.label(
            egui::RichText::new("Arrow keys to navigate")
                .size(11.0)
                .color(egui::Color32::from_rgb(140, 130, 120)),
        );
        ui.label(
            egui::RichText::new("Esc to return to now")
                .size(11.0)
                .color(egui::Color32::from_rgb(140, 130, 120)),
        );
    });
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

/// Draw the inspect tooltip at the given screen position
#[allow(dead_code)]
pub fn draw_inspect_tooltip(
    ctx: &egui::Context,
    screen_pos: egui::Pos2,
    time_str: &str,
    is_gap: bool,
    is_overlap: bool,
    is_pinned: bool,
) -> InspectResult {
    let mut result = InspectResult::default();

    egui::Window::new("Inspect")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_pos(screen_pos)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if is_pinned {
                    ui.colored_label(egui::Color32::from_rgb(120, 180, 220), "📌");
                }
                ui.label(
                    egui::RichText::new(time_str)
                        .size(14.0)
                        .color(egui::Color32::from_rgb(245, 230, 211)),
                );
            });

            if is_gap {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 107, 53),
                    "⚠ Nonexistent (DST gap)",
                );
            } else if is_overlap {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 179, 71),
                    "⚠ Ambiguous (DST overlap)",
                );
            }

            ui.horizontal(|ui| {
                if ui.small_button(if is_pinned { "Unpin" } else { "Pin" }).clicked() {
                    result.toggle_pin = true;
                }
                if ui.small_button("Return to Now").clicked() {
                    result.return_to_now = true;
                }
            });
        });

    result
}

/// Draw keyboard help overlay
#[allow(dead_code)]
pub fn draw_help_overlay(ctx: &egui::Context) {
    egui::Window::new("Keyboard Shortcuts")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .show(ctx, |ui| {
            ui.label("← → : Move by minute");
            ui.label("Shift+← → : Move by hour");
            ui.label("Enter : Pin/unpin inspection");
            ui.label("Esc : Return to now");
            ui.label("/ : Search timezone");
        });
}

