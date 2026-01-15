//! UI module for the Temporal Grammar Clock
//!
//! Provides the sidebar panel with timezone picker, mode toggles,
//! diagram description, and accessibility controls using egui.

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

/// Result of sidebar interactions
#[derive(Default)]
pub struct SidebarResult {
    /// Set a new timezone
    pub set_timezone: Option<Tz>,
    /// Toggle favorite status
    pub toggle_favorite: Option<Tz>,
    /// Toggle decode mode
    pub toggle_decode_mode: bool,
    /// Toggle explicit mode
    pub toggle_explicit_mode: bool,
    /// Toggle reduced motion
    pub toggle_reduced_motion: bool,
    /// Open help panel
    pub open_help: bool,
    /// Step time by seconds (positive = forward, negative = backward)
    pub step_time: Option<i64>,
    /// Return to live time
    pub return_to_live: bool,
}

/// Draw the sidebar panel
pub fn draw_sidebar(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    selected_zone: Tz,
    favorites: &[Tz],
    time_data: &TimeData,
    decode_mode: bool,
    explicit_mode: bool,
    reduced_motion: bool,
    diagram_description: &str,
    is_live: bool,
) -> SidebarResult {
    let mut result = SidebarResult::default();

    // Apply temporal grammar theme
    let mut style = (*ctx.style()).clone();
    style.visuals.window_fill = egui::Color32::from_rgb(18, 16, 26);
    style.visuals.panel_fill = egui::Color32::from_rgb(18, 16, 26);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(28, 24, 40);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 30, 50);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 45, 70);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(65, 55, 90);
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(200, 200, 210));
    ctx.set_style(style);

    egui::SidePanel::right("sidebar")
        .resizable(false)
        .min_width(260.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);

            // Title
            ui.heading(
                egui::RichText::new("Temporal Grammar")
                    .color(egui::Color32::from_rgb(100, 200, 255))
                    .size(18.0),
            );
            ui.add_space(10.0);

            // Timezone section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ TIMEZONE")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                // Current timezone display
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format_zone_name(selected_zone))
                            .size(13.0)
                            .color(egui::Color32::from_rgb(220, 220, 230)),
                    );
                    ui.label(
                        egui::RichText::new(format!("({})", time_data.tz_abbrev))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(140, 140, 150)),
                    );
                });

                ui.add_space(5.0);

                // Open picker button
                if ui
                    .button(egui::RichText::new("Change Zone (Z)").size(12.0))
                    .clicked()
                {
                    picker_state.open();
                }

                // System timezone shortcut
                if ui
                    .button(egui::RichText::new("Use System TZ").size(12.0))
                    .clicked()
                {
                    if let Some(sys_tz) = system_timezone() {
                        result.set_timezone = Some(sys_tz);
                    }
                }
            });

            ui.add_space(10.0);

            // DST section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ DST STATUS")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                // DST indicator
                if time_data.is_dst {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Status:")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 150)),
                        );
                        ui.label(
                            egui::RichText::new("DST Active")
                                .color(egui::Color32::from_rgb(255, 180, 100)),
                        );
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Status:")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(140, 140, 150)),
                        );
                        ui.label(
                            egui::RichText::new("Standard Time")
                                .color(egui::Color32::from_rgb(140, 140, 150)),
                        );
                    });
                }

                // DST change warning
                match &time_data.dst_change {
                    DstChange::Upcoming { instant, delta_minutes } => {
                        let hours_until = (*instant - chrono::Utc::now()).num_hours();
                        let direction = if *delta_minutes > 0 {
                            "spring forward"
                        } else {
                            "fall back"
                        };
                        ui.add_space(3.0);
                        ui.label(
                            egui::RichText::new(format!("⚠ Change in {}h ({})", hours_until, direction))
                                .color(egui::Color32::from_rgb(255, 140, 60)),
                        );
                    }
                    DstChange::JustOccurred { delta_minutes, .. } => {
                        let direction = if *delta_minutes > 0 {
                            "sprang forward"
                        } else {
                            "fell back"
                        };
                        ui.add_space(3.0);
                        ui.label(
                            egui::RichText::new(format!("ℹ Clocks {} recently", direction))
                                .color(egui::Color32::from_rgb(100, 180, 255)),
                        );
                    }
                    DstChange::None => {}
                }
            });

            ui.add_space(10.0);

            // Time control section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ TIME CONTROL")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                // Live/Paused indicator
                if is_live {
                    ui.label(
                        egui::RichText::new("● LIVE")
                            .color(egui::Color32::from_rgb(100, 255, 150)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("⏸ PAUSED")
                            .color(egui::Color32::from_rgb(255, 180, 100)),
                    );
                    // Show the frozen time
                    let time_str = format!(
                        "{:02}:{:02}:{:02} {}",
                        time_data.hour12, time_data.minute, time_data.second, time_data.meridiem
                    );
                    ui.label(
                        egui::RichText::new(time_str)
                            .size(11.0)
                            .color(egui::Color32::from_rgb(180, 180, 190)),
                    );
                }

                ui.add_space(5.0);

                // Step controls
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("◀◀").size(12.0)).clicked() {
                        result.step_time = Some(-3600); // -1 hour
                    }
                    if ui.button(egui::RichText::new("◀").size(12.0)).clicked() {
                        result.step_time = Some(-60); // -1 minute
                    }
                    if ui.button(egui::RichText::new("‹").size(12.0)).clicked() {
                        result.step_time = Some(-1); // -1 second
                    }
                    if ui.button(egui::RichText::new("›").size(12.0)).clicked() {
                        result.step_time = Some(1); // +1 second
                    }
                    if ui.button(egui::RichText::new("▶").size(12.0)).clicked() {
                        result.step_time = Some(60); // +1 minute
                    }
                    if ui.button(egui::RichText::new("▶▶").size(12.0)).clicked() {
                        result.step_time = Some(3600); // +1 hour
                    }
                });

                ui.add_space(3.0);

                // Return to live button
                if !is_live {
                    if ui
                        .button(
                            egui::RichText::new("Return to Live (L)")
                                .size(12.0)
                                .color(egui::Color32::from_rgb(100, 255, 150)),
                        )
                        .clicked()
                    {
                        result.return_to_live = true;
                    }
                }

                ui.label(
                    egui::RichText::new("[ ] step sec  |  Shift: min  |  Ctrl: hr")
                        .size(9.0)
                        .color(egui::Color32::from_rgb(100, 100, 110)),
                );
            });

            ui.add_space(10.0);

            // Mode toggles section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ VIEW MODES")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                // Decode mode toggle
                let decode_text = if decode_mode {
                    egui::RichText::new("[D] Decode Mode: ON")
                        .color(egui::Color32::from_rgb(180, 255, 180))
                } else {
                    egui::RichText::new("[D] Decode Mode: OFF")
                        .color(egui::Color32::from_rgb(140, 140, 150))
                };
                if ui.button(decode_text).clicked() {
                    result.toggle_decode_mode = true;
                }

                ui.add_space(3.0);

                // Explicit mode toggle
                let explicit_text = if explicit_mode {
                    egui::RichText::new("Explicit Mode: ON")
                        .color(egui::Color32::from_rgb(180, 255, 180))
                } else {
                    egui::RichText::new("Explicit Mode: OFF")
                        .color(egui::Color32::from_rgb(140, 140, 150))
                };
                if ui.button(explicit_text).clicked() {
                    result.toggle_explicit_mode = true;
                }
                ui.label(
                    egui::RichText::new("(Standard time display)")
                        .size(10.0)
                        .color(egui::Color32::from_rgb(100, 100, 110)),
                );
            });

            ui.add_space(10.0);

            // Accessibility section
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ ACCESSIBILITY")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                // Reduced motion toggle
                let mut reduced = reduced_motion;
                if ui
                    .checkbox(&mut reduced, egui::RichText::new("Reduced motion").size(12.0))
                    .changed()
                {
                    result.toggle_reduced_motion = true;
                }

                ui.add_space(5.0);

                // Help button
                if ui
                    .button(egui::RichText::new("[?] How to read this clock").size(12.0))
                    .clicked()
                {
                    result.open_help = true;
                }
            });

            ui.add_space(10.0);

            // Diagram description section (for accessibility)
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ DIAGRAM STATE")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                // Wrap the description text
                ui.label(
                    egui::RichText::new(diagram_description)
                        .size(10.0)
                        .color(egui::Color32::from_rgb(160, 160, 170)),
                );
            });

            ui.add_space(10.0);

            // Keyboard shortcuts
            ui.group(|ui| {
                ui.label(
                    egui::RichText::new("▸ SHORTCUTS")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
                ui.add_space(5.0);

                let shortcuts = [
                    ("Space", "Hold to reveal time"),
                    ("D", "Toggle decode mode"),
                    ("Z", "Open timezone picker"),
                    ("?", "Help panel"),
                    ("[ / ]", "Step time back/fwd"),
                    ("L", "Return to live"),
                    ("Tab", "Cycle focus"),
                    ("Esc", "Close panels"),
                ];

                for (key, desc) in shortcuts {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{:>6}", key))
                                .size(10.0)
                                .color(egui::Color32::from_rgb(100, 200, 255))
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(desc)
                                .size(10.0)
                                .color(egui::Color32::from_rgb(120, 120, 130)),
                        );
                    });
                }
            });

            // Truth anchor status at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Hold Space or click to reveal exact time")
                        .size(9.0)
                        .color(egui::Color32::from_rgb(80, 80, 90)),
                );
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

    egui::Window::new("Select Timezone")
        .collapsible(false)
        .resizable(true)
        .default_width(380.0)
        .default_height(450.0)
        .show(ctx, |ui| {
            // Search field
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Search:")
                        .color(egui::Color32::from_rgb(100, 200, 255)),
                );
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
                ui.collapsing(
                    egui::RichText::new("★ Favorites")
                        .color(egui::Color32::from_rgb(255, 200, 100)),
                    |ui| {
                        for &tz in favorites {
                            ui.horizontal(|ui| {
                                if ui
                                    .button(egui::RichText::new(format_zone_name(tz)).size(12.0))
                                    .clicked()
                                {
                                    result.select_zone = Some(tz);
                                }
                                if ui.small_button("★").clicked() {
                                    result.toggle_favorite = Some(tz);
                                }
                            });
                        }
                    },
                );
                ui.separator();
            }

            // System timezone shortcut
            if ui
                .button(egui::RichText::new("📍 Use System Timezone").size(12.0))
                .clicked()
            {
                if let Some(sys_tz) = system_timezone() {
                    result.select_zone = Some(sys_tz);
                }
            }

            ui.separator();

            // Search results count
            ui.label(
                egui::RichText::new(format!("{} results", picker_state.search_results.len()))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(120, 120, 130)),
            );

            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for &tz in picker_state.search_results.iter().take(100) {
                        let is_favorite = favorites.contains(&tz);
                        ui.horizontal(|ui| {
                            if ui
                                .button(egui::RichText::new(format_zone_name(tz)).size(12.0))
                                .clicked()
                            {
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

            if ui
                .button(egui::RichText::new("Close (Esc)").size(12.0))
                .clicked()
            {
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

