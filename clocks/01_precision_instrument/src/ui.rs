//! UI module - egui timezone picker and DST status card
//!
//! Provides the interactive UI components using nannou_egui.

use chrono_tz::Tz;
use nannou_egui::egui;
use shared::{search_timezones, DstChange, TimeData};

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

/// Result of UI interactions
pub struct UiResult {
    /// If Some, the user selected a new timezone
    pub selected_tz: Option<Tz>,
    /// If Some, toggle favorite status for this timezone
    pub toggle_favorite: Option<Tz>,
    /// If true, close the picker
    pub close_picker: bool,
    /// If true, toggle reduced motion setting
    #[allow(dead_code)]
    pub toggle_reduced_motion: bool,
}

impl Default for UiResult {
    fn default() -> Self {
        Self {
            selected_tz: None,
            toggle_favorite: None,
            close_picker: false,
            toggle_reduced_motion: false,
        }
    }
}

/// Draw the timezone picker overlay
pub fn draw_timezone_picker(
    ctx: &egui::Context,
    picker_state: &mut PickerState,
    current_tz: Tz,
    favorites: &[Tz],
) -> UiResult {
    let mut result = UiResult::default();

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
            let _search_response = ui.horizontal(|ui| {
                ui.label("Search:");
                let response = ui.text_edit_singleline(&mut picker_state.search_query);
                if picker_state.should_focus_search {
                    response.request_focus();
                    picker_state.should_focus_search = false;
                }
                if response.changed() {
                    picker_state.update_search();
                }
                response
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
            ui.label(format!("{} time zones found", picker_state.search_results.len()));
            
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

/// Draw the DST status card
pub fn draw_dst_status_card(ctx: &egui::Context, time_data: &TimeData) {
    egui::Window::new("DST Status")
        .collapsible(true)
        .resizable(false)
        .default_width(250.0)
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .show(ctx, |ui| {
            // Current DST status
            ui.horizontal(|ui| {
                ui.label("Status:");
                if time_data.is_dst {
                    ui.colored_label(egui::Color32::from_rgb(0, 212, 255), "Daylight Saving Time");
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
                DstChange::Upcoming { instant, delta_minutes } => {
                    let direction = if *delta_minutes > 0 { "forward" } else { "back" };
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 180, 0),
                        format!("⚠ Upcoming DST Change"),
                    );
                    ui.label(format!(
                        "Clocks will move {} by {} minutes",
                        direction,
                        delta_minutes.abs()
                    ));
                    ui.label(format!("At: {}", instant.format("%Y-%m-%d %H:%M UTC")));
                }
                DstChange::JustOccurred { instant, delta_minutes } => {
                    let direction = if *delta_minutes > 0 { "forward" } else { "back" };
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 212, 255),
                        format!("ℹ Recent DST Change"),
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

/// Draw the favorites chips row
pub fn draw_favorites_chips(
    ctx: &egui::Context,
    favorites: &[Tz],
    current_tz: Tz,
) -> Option<Tz> {
    let mut selected = None;

    if favorites.is_empty() {
        return None;
    }

    egui::TopBottomPanel::bottom("favorites_panel")
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Quick Select:");
                for &tz in favorites {
                    let is_current = tz == current_tz;
                    // Get just the city name from the timezone
                    let display_name = tz
                        .name()
                        .rsplit('/')
                        .next()
                        .unwrap_or(tz.name())
                        .replace('_', " ");
                    
                    if ui.selectable_label(is_current, &display_name).clicked() {
                        selected = Some(tz);
                    }
                }
            });
        });

    selected
}

/// Draw the settings panel
pub fn draw_settings_panel(ctx: &egui::Context, reduced_motion: &mut bool) -> bool {
    let mut changed = false;

    egui::Window::new("Settings")
        .collapsible(true)
        .resizable(false)
        .default_width(200.0)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -50.0])
        .show(ctx, |ui| {
            if ui.checkbox(reduced_motion, "Reduced Motion").changed() {
                changed = true;
            }
            ui.label("Disables continuous animations");
        });

    changed
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
            });
        });

    clicked
}
