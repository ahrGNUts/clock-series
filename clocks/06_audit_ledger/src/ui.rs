//! UI module for the Audit Ledger Clock
//!
//! Provides the sidebar with timezone picker, DST insights panel,
//! time range filter, and density controls using egui.

use chrono_tz::Tz;
use nannou_egui::egui;
use shared::{search_timezones, system_timezone, DstChange, TimeData};

use crate::ledger::{LedgerState, TimeRangeFilter};
use crate::TextDensity;

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
    /// Set time range filter
    pub set_time_range: Option<TimeRangeFilter>,
    /// Set text density
    pub set_density: Option<TextDensity>,
    /// Set reduced motion
    pub set_reduced_motion: Option<bool>,
}

/// Draw the sidebar panel
pub fn draw_sidebar(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    selected_zone: Tz,
    favorites: &[Tz],
    time_data: &TimeData,
    ledger: &LedgerState,
    text_density: TextDensity,
    reduced_motion: bool,
) -> SidebarResult {
    let mut result = SidebarResult::default();

    // Apply terminal-style theme
    let mut style = (*ctx.style()).clone();
    style.visuals.window_fill = egui::Color32::from_rgb(15, 20, 25);
    style.visuals.panel_fill = egui::Color32::from_rgb(15, 20, 25);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 28, 35);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 35, 45);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 50, 60);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(45, 65, 80);
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(51, 255, 102));
    ctx.set_style(style);

    egui::SidePanel::right("sidebar")
        .resizable(false)
        .min_width(280.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);

            // Title
            ui.heading(egui::RichText::new("╔══ CONTROLS ══╗").color(egui::Color32::from_rgb(51, 255, 102)));
            ui.add_space(10.0);

            // Timezone section
            ui.group(|ui| {
                ui.label(egui::RichText::new("▸ TIMEZONE").size(14.0).color(egui::Color32::from_rgb(51, 255, 102)));
                ui.add_space(5.0);

                // Current timezone display
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format_zone_name(selected_zone))
                            .size(13.0)
                            .color(egui::Color32::from_rgb(200, 255, 200)),
                    );
                    ui.label(
                        egui::RichText::new(format!("({})", time_data.tz_abbrev))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(100, 150, 100)),
                    );
                });

                ui.add_space(5.0);

                // Open picker button
                if ui.button(egui::RichText::new("Change Zone (T)").size(12.0)).clicked() {
                    picker_state.open();
                }

                // System timezone shortcut
                if ui.button(egui::RichText::new("Use System TZ").size(12.0)).clicked() {
                    if let Some(sys_tz) = system_timezone() {
                        result.set_timezone = Some(sys_tz);
                    }
                }
            });

            ui.add_space(10.0);

            // DST Insights section
            ui.group(|ui| {
                ui.label(egui::RichText::new("▸ DST INSIGHTS").size(14.0).color(egui::Color32::from_rgb(51, 255, 102)));
                ui.add_space(5.0);

                // DST active indicator
                let dst_text = if time_data.is_dst {
                    egui::RichText::new("● Daylight Saving Time ACTIVE")
                        .color(egui::Color32::from_rgb(255, 200, 100))
                } else {
                    egui::RichText::new("○ Standard Time")
                        .color(egui::Color32::from_rgb(100, 150, 100))
                };
                ui.label(dst_text);

                ui.add_space(5.0);

                // Plain language DST explanation
                let dst_explanation = get_dst_explanation(time_data);
                ui.label(
                    egui::RichText::new(dst_explanation)
                        .size(11.0)
                        .color(egui::Color32::from_rgb(150, 180, 150)),
                );

                // DST warning
                match &time_data.dst_change {
                    DstChange::Upcoming { instant, delta_minutes } => {
                        let hours_until = (*instant - chrono::Utc::now()).num_hours();
                        let direction = if *delta_minutes > 0 {
                            "spring forward"
                        } else {
                            "fall back"
                        };
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(format!("⚠ DST change in {}h ({})", hours_until, direction))
                                .color(egui::Color32::from_rgb(255, 150, 80)),
                        );
                    }
                    DstChange::JustOccurred { delta_minutes, .. } => {
                        let direction = if *delta_minutes > 0 {
                            "sprang forward"
                        } else {
                            "fell back"
                        };
                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new(format!("ℹ Clocks {} recently", direction))
                                .color(egui::Color32::from_rgb(100, 180, 255)),
                        );
                    }
                    DstChange::None => {}
                }
            });

            ui.add_space(10.0);

            // Time Range Filter section
            ui.group(|ui| {
                ui.label(egui::RichText::new("▸ TIME RANGE").size(14.0).color(egui::Color32::from_rgb(51, 255, 102)));
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    for range in TimeRangeFilter::all() {
                        let is_selected = ledger.time_range == *range;
                        let text = if is_selected {
                            egui::RichText::new(format!("[{}]", range.label()))
                                .color(egui::Color32::from_rgb(51, 255, 102))
                        } else {
                            egui::RichText::new(range.label())
                                .color(egui::Color32::from_rgb(100, 150, 100))
                        };

                        if ui.button(text).clicked() {
                            result.set_time_range = Some(*range);
                        }
                    }
                });

                ui.add_space(3.0);
                ui.label(
                    egui::RichText::new(format!("{} entries in buffer", ledger.entries.len()))
                        .size(10.0)
                        .color(egui::Color32::from_rgb(80, 120, 80)),
                );
            });

            ui.add_space(10.0);

            // Accessibility section
            ui.group(|ui| {
                ui.label(egui::RichText::new("▸ ACCESSIBILITY").size(14.0).color(egui::Color32::from_rgb(51, 255, 102)));
                ui.add_space(5.0);

                // Text density
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Density:").size(12.0));
                    if ui.button(egui::RichText::new(text_density.label()).size(12.0)).clicked() {
                        result.set_density = Some(text_density.cycle());
                    }
                });

                // Reduced motion toggle
                let mut reduced = reduced_motion;
                if ui.checkbox(&mut reduced, egui::RichText::new("Reduced motion").size(12.0)).changed() {
                    result.set_reduced_motion = Some(reduced);
                }
            });

            ui.add_space(10.0);

            // Keyboard shortcuts help
            ui.group(|ui| {
                ui.label(egui::RichText::new("▸ SHORTCUTS").size(14.0).color(egui::Color32::from_rgb(51, 255, 102)));
                ui.add_space(5.0);

                let shortcuts = [
                    ("T", "Open timezone picker"),
                    ("L", "Return to live"),
                    ("J/K", "Scroll down/up"),
                    ("[/]", "Collapse/expand"),
                    ("Esc", "Close/return"),
                ];

                for (key, desc) in shortcuts {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{:>5}", key))
                                .size(10.0)
                                .color(egui::Color32::from_rgb(80, 180, 255))
                                .monospace(),
                        );
                        ui.label(
                            egui::RichText::new(desc)
                                .size(10.0)
                                .color(egui::Color32::from_rgb(100, 150, 100)),
                        );
                    });
                }
            });

            // Ledger status at bottom
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(10.0);

                let status_text = if ledger.is_live {
                    egui::RichText::new("● LIVE")
                        .color(egui::Color32::from_rgb(51, 255, 102))
                } else {
                    egui::RichText::new("○ PAUSED")
                        .color(egui::Color32::from_rgb(255, 150, 80))
                };
                ui.label(status_text);
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

    egui::Window::new("╔══ SELECT TIMEZONE ══╗")
        .collapsible(false)
        .resizable(true)
        .default_width(400.0)
        .default_height(500.0)
        .show(ctx, |ui| {
            // Search field
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Search:").color(egui::Color32::from_rgb(51, 255, 102)));
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
                ui.collapsing(egui::RichText::new("★ Favorites").color(egui::Color32::from_rgb(255, 200, 100)), |ui| {
                    for &tz in favorites {
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new(format_zone_name(tz)).size(12.0)).clicked() {
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
            if ui.button(egui::RichText::new("📍 Use System Timezone").size(12.0)).clicked() {
                if let Some(sys_tz) = system_timezone() {
                    result.select_zone = Some(sys_tz);
                }
            }

            ui.separator();

            // Search results
            ui.label(
                egui::RichText::new(format!("{} results", picker_state.search_results.len()))
                    .size(11.0)
                    .color(egui::Color32::from_rgb(100, 150, 100)),
            );

            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for &tz in picker_state.search_results.iter().take(100) {
                        let is_favorite = favorites.contains(&tz);
                        ui.horizontal(|ui| {
                            if ui.button(egui::RichText::new(format_zone_name(tz)).size(12.0)).clicked() {
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

            if ui.button(egui::RichText::new("Close (Esc)").size(12.0)).clicked() {
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

/// Get a plain language explanation of DST for the current timezone
fn get_dst_explanation(time_data: &TimeData) -> String {
    // This is a simplified explanation - in a full implementation,
    // you'd query the timezone database for actual transition dates
    let year = time_data.year;

    if time_data.is_dst {
        format!(
            "DST is active. Clocks will fall back\n\
             1 hour in autumn {}.",
            year
        )
    } else {
        // Check if we're before or after spring DST
        let month = time_data.month;
        if month < 3 || month > 11 {
            format!(
                "Standard time. Clocks will spring\n\
                 forward 1 hour in March {}.",
                if month > 11 { year + 1 } else { year }
            )
        } else {
            format!(
                "Standard time. DST ended for {}.\n\
                 Next DST starts March {}.",
                year,
                year + 1
            )
        }
    }
}

