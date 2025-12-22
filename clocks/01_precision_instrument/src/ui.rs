//! UI module - egui timezone picker and DST status card
//!
//! Provides the interactive UI components using nannou_egui.

use chrono_tz::Tz;
use nannou_egui::egui;
use shared::{all_timezones, search_timezones, DstChange, TimeData};

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
    /// Currently selected index for keyboard navigation
    pub selected_index: Option<usize>,
    /// Total timezone count (for "showing X of Y")
    total_count: usize,
}

impl PickerState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.search_query.clear();
        self.search_results = search_timezones("");
        self.total_count = all_timezones().len();
        self.should_focus_search = true;
        self.selected_index = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.search_query.clear();
        self.search_results.clear();
        self.selected_index = None;
    }

    pub fn update_search(&mut self) {
        self.search_results = search_timezones(&self.search_query);
        // Reset selection when search changes
        self.selected_index = if self.search_results.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    /// Move the keyboard selection up or down
    pub fn move_selection(&mut self, delta: i32) {
        if self.search_results.is_empty() {
            self.selected_index = None;
            return;
        }

        let len = self.search_results.len();
        self.selected_index = Some(match self.selected_index {
            None => {
                if delta > 0 { 0 } else { len - 1 }
            }
            Some(idx) => {
                let new_idx = idx as i32 + delta;
                if new_idx < 0 {
                    len - 1
                } else if new_idx >= len as i32 {
                    0
                } else {
                    new_idx as usize
                }
            }
        });
    }

    /// Get the currently selected timezone (for Enter key)
    pub fn get_selected(&self) -> Option<Tz> {
        self.selected_index
            .and_then(|idx| self.search_results.get(idx).copied())
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

            // Results list with count
            ui.label(format!(
                "Showing {} of {} time zones",
                picker_state.search_results.len(),
                picker_state.total_count
            ));
            
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for (idx, &tz) in picker_state.search_results.iter().enumerate() {
                        let is_current = tz == current_tz;
                        let is_favorite = favorites.contains(&tz);
                        let is_keyboard_selected = picker_state.selected_index == Some(idx);
                        
                        ui.horizontal(|ui| {
                            // Star button for favorites
                            let star = if is_favorite { "★" } else { "☆" };
                            if ui.small_button(star).clicked() {
                                result.toggle_favorite = Some(tz);
                            }
                            
                            // Timezone name - highlight if keyboard selected
                            let label = if is_current {
                                format!("{} ◀", tz.name())
                            } else {
                                tz.name().to_string()
                            };
                            
                            let response = ui.selectable_label(
                                is_current || is_keyboard_selected,
                                &label,
                            );
                            
                            // Scroll to keyboard-selected item
                            if is_keyboard_selected {
                                response.scroll_to_me(Some(egui::Align::Center));
                            }
                            
                            if response.clicked() {
                                result.selected_tz = Some(tz);
                                result.close_picker = true;
                            }
                        });
                    }
                });

            ui.separator();

            // Keyboard hint
            ui.label("↑↓ Navigate · Enter Select · Esc Close");

            ui.separator();

            // Close button
            if ui.button("Close").clicked() {
                result.close_picker = true;
            }
        });

    // Handle Enter key in picker (Escape is handled in main.rs key_pressed)
    ctx.input(|i| {
        if i.key_pressed(egui::Key::Enter) {
            if let Some(tz) = picker_state.get_selected() {
                result.selected_tz = Some(tz);
                result.close_picker = true;
            }
        }
    });

    result
}

/// Draw the DST status card
pub fn draw_dst_status_card(ctx: &egui::Context, time_data: &TimeData, selected_tz: Tz) {
    egui::Window::new("DST Status")
        .collapsible(true)
        .resizable(false)
        .default_width(280.0)
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
                        "⚠ Upcoming DST Change".to_string(),
                    );
                    ui.label(format!(
                        "Clocks will move {} by {} minutes",
                        direction,
                        delta_minutes.abs()
                    ));
                    // Show in local time
                    let local_time = instant.with_timezone(&selected_tz);
                    ui.label(format!("At: {}", local_time.format("%b %d, %Y %I:%M %p (local)")));
                }
                DstChange::JustOccurred { instant, delta_minutes } => {
                    let direction = if *delta_minutes > 0 { "forward" } else { "back" };
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 212, 255),
                        "ℹ Recent DST Change".to_string(),
                    );
                    ui.label(format!(
                        "Clocks moved {} by {} minutes",
                        direction,
                        delta_minutes.abs()
                    ));
                    // Show in local time
                    let local_time = instant.with_timezone(&selected_tz);
                    ui.label(format!("At: {}", local_time.format("%b %d, %Y %I:%M %p (local)")));
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
            ui.separator();
            ui.label("Press R to toggle");
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
                    .on_hover_text("Click to change time zone (or press Space)")
                    .clicked()
                {
                    clicked = true;
                }
            });
        });

    clicked
}
