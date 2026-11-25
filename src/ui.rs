use crate::state::TodoItem;
use eframe::egui::{self, Color32, RichText, WindowLevel};

pub(crate) const BUTTON_WIDTH: f32 = 108.0;
pub(crate) const MODE_BUTTON_WIDTH: f32 = 96.0;
pub(crate) const POMODORO_PANEL_SIZE: f32 = 320.0;
pub(crate) const POMODORO_PANEL_MARGIN: f32 = 14.0;
pub(crate) const POMODORO_TOP_PADDING: f32 = 8.0;
pub(crate) const PROGRESS_CIRCLE_SIZE: f32 = 8.0;
pub(crate) const FOCUS_SESSION_CIRCLE_SIZE: f32 = 7.0;
pub(crate) const BUTTON_PADDING: f32 = 8.0;
pub(crate) const BUTTON_BASE_HEIGHT: f32 = 26.0;
pub(crate) const TIMER_GROUP_PADDING: i8 = 32;
pub(crate) const TODO_INPUT_BOTTOM_MARGIN: f32 = 80.0;
pub(crate) const TODO_INPUT_HEIGHT: f32 = 44.0;
pub(crate) const TODO_LIST_MIN_HEIGHT: f32 = 84.0;
pub(crate) const TODO_ROW_HEIGHT: f32 = 28.0;
pub(crate) const TODO_ROW_CORNER_RADIUS: f32 = 6.0;

pub(crate) fn centered_left_pad(available_width: f32, content_width: f32) -> f32 {
    ((available_width - content_width) / 2.0).max(0.0)
}

pub(crate) fn centered_buttons<R>(
    ui: &mut egui::Ui,
    count: usize,
    button_width: f32,
    add_buttons: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let spacing = ui.spacing().item_spacing.x;
    let row_width = count as f32 * button_width + spacing * count.saturating_sub(1) as f32;
    let left_pad = centered_left_pad(ui.available_width(), row_width);
    ui.horizontal(|ui| {
        ui.add_space(left_pad);
        add_buttons(ui)
    })
    .inner
}

pub(crate) fn pomodoro_panel_side(available_width: f32) -> f32 {
    (available_width - 2.0 * POMODORO_PANEL_MARGIN).clamp(0.0, POMODORO_PANEL_SIZE)
}

pub(crate) fn pomodoro_top_space(side: f32) -> f32 {
    POMODORO_TOP_PADDING.min(side)
}

pub(crate) fn pomodoro_section_width(available_width: f32) -> f32 {
    pomodoro_panel_side(available_width) + 2.0 * POMODORO_PANEL_MARGIN
}

pub(crate) fn start_button_size() -> egui::Vec2 {
    egui::vec2(
        BUTTON_WIDTH + 2.0 * BUTTON_PADDING,
        BUTTON_BASE_HEIGHT + 2.0 * BUTTON_PADDING,
    )
}

pub(crate) fn timer_group_margin() -> egui::Margin {
    egui::Margin::symmetric(0, TIMER_GROUP_PADDING)
}

pub(crate) fn window_inner_size() -> [f32; 2] {
    [390.0, 800.0]
}

pub(crate) fn window_level(always_on_top: bool) -> WindowLevel {
    if always_on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

pub(crate) fn pin_button_label(always_on_top: bool) -> &'static str {
    if always_on_top {
        "Unpin window"
    } else {
        "Pin on top"
    }
}

pub(crate) fn focus_button_label(focused: bool) -> &'static str {
    if focused {
        "◉"
    } else {
        "◎"
    }
}

pub(crate) fn todo_list_max_height(available_height: f32) -> f32 {
    available_height.max(TODO_LIST_MIN_HEIGHT)
}

pub(crate) fn todo_input_bottom_margin() -> f32 {
    TODO_INPUT_BOTTOM_MARGIN
}

pub(crate) fn todo_input_background() -> Color32 {
    Color32::from_gray(70)
}

pub(crate) fn todo_input_margin() -> egui::Margin {
    egui::Margin::symmetric(14, 10)
}

pub(crate) fn todo_input_align() -> egui::Align2 {
    egui::Align2::CENTER_CENTER
}

pub(crate) fn todo_section_width(available_width: f32) -> f32 {
    pomodoro_section_width(available_width).min(available_width)
}

pub(crate) fn todo_input_width(available_width: f32) -> f32 {
    todo_section_width(available_width)
}

pub(crate) fn todo_input_height() -> f32 {
    TODO_INPUT_HEIGHT
}

pub(crate) fn todo_list_auto_shrink() -> egui::Vec2b {
    egui::Vec2b::new(false, true)
}

pub(crate) fn todo_list_scroll_area(available_height: f32) -> egui::ScrollArea {
    egui::ScrollArea::vertical()
        .max_height(todo_list_max_height(available_height))
        .auto_shrink(todo_list_auto_shrink())
}

pub(crate) fn todo_footer_height() -> f32 {
    todo_input_height() + todo_input_bottom_margin()
}

pub(crate) fn todo_content_rect(window: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        window.min,
        egui::pos2(window.max.x, window.max.y - todo_footer_height()),
    )
}

pub(crate) fn todo_footer_rect(window: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(window.min.x, window.max.y - todo_footer_height()),
        window.max,
    )
}

pub(crate) fn todo_input_shadow() -> egui::Shadow {
    egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(90),
    }
}

pub(crate) fn todo_input_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(todo_input_background())
        .inner_margin(todo_input_margin())
        .corner_radius(12.0)
        .shadow(todo_input_shadow())
}

pub(crate) fn should_submit_todo(lost_focus: bool, enter_pressed: bool) -> bool {
    lost_focus && enter_pressed
}

pub(crate) fn todo_text(item: &TodoItem) -> RichText {
    let text = RichText::new(item.text.as_str());
    if item.done {
        text.strikethrough().weak()
    } else {
        text
    }
}

pub(crate) fn todo_row_background(hovered: bool) -> Color32 {
    if hovered {
        Color32::from_white_alpha(18)
    } else {
        Color32::TRANSPARENT
    }
}

pub(crate) fn todo_row_hovered(ui: &egui::Ui, rect: egui::Rect) -> bool {
    ui.rect_contains_pointer(rect)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TodoRowResponse {
    pub(crate) toggled: bool,
    pub(crate) removed: bool,
    pub(crate) focus_toggled: bool,
}

pub(crate) fn todo_row(ui: &mut egui::Ui, item: &TodoItem, focused: bool) -> TodoRowResponse {
    let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), TODO_ROW_HEIGHT));
    let hovered = todo_row_hovered(ui, rect);
    let background = todo_row_background(hovered || focused);
    ui.painter()
        .rect_filled(rect, TODO_ROW_CORNER_RADIUS, background);

    let mut row_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut checked = item.done;
    let toggled = row_ui.checkbox(&mut checked, todo_text(item)).changed();

    focus_sessions_indicator(&mut row_ui, item.sessions);

    let mut removed = false;
    let mut focus_toggled = false;
    row_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let delete = ui.add(
            egui::Button::new(RichText::new("✕").size(14.0).weak())
                .frame(false)
                .small(),
        );
        if delete.clicked() {
            removed = true;
        }
        let focus = ui.add(
            egui::Button::new(RichText::new(focus_button_label(focused)).size(14.0))
                .frame(false)
                .small()
                .selected(focused),
        );
        if focus.clicked() {
            focus_toggled = true;
        }
    });

    TodoRowResponse {
        toggled,
        removed,
        focus_toggled,
    }
}

pub(crate) fn progress_row_width(count: usize, circle_size: f32, spacing: f32) -> f32 {
    count as f32 * circle_size + spacing * count.saturating_sub(1) as f32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CircleState {
    Filled,
    Empty,
}

pub(crate) fn progress_circles(completed: u32, total: u32) -> Vec<CircleState> {
    if total == 0 {
        return Vec::new();
    }
    let filled = completed % total;
    (0..total)
        .map(|index| {
            if index < filled {
                CircleState::Filled
            } else {
                CircleState::Empty
            }
        })
        .collect()
}

pub(crate) fn progress_indicator(ui: &mut egui::Ui, completed: u32, total: u32) {
    ui.horizontal(|ui| {
        let states = progress_circles(completed, total);
        let spacing = ui.spacing().item_spacing.x;
        let row_width = progress_row_width(states.len(), PROGRESS_CIRCLE_SIZE, spacing);
        ui.add_space(((ui.available_width() - row_width) / 2.0).max(0.0));
        let active_fill = ui.visuals().selection.bg_fill;
        for state in states {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(PROGRESS_CIRCLE_SIZE, PROGRESS_CIRCLE_SIZE),
                egui::Sense::hover(),
            );
            let center = rect.center();
            let radius = PROGRESS_CIRCLE_SIZE / 2.0;
            match state {
                CircleState::Filled => {
                    ui.painter().circle_filled(center, radius, active_fill);
                }
                CircleState::Empty => {
                    ui.painter().circle_stroke(
                        center,
                        radius - 1.0,
                        egui::Stroke::new(1.5, Color32::from_gray(100)),
                    );
                }
            }
        }
    });
}

pub(crate) fn focus_session_circles(count: u32) -> Vec<CircleState> {
    (0..count).map(|_| CircleState::Filled).collect()
}

pub(crate) fn focus_sessions_indicator(ui: &mut egui::Ui, count: u32) {
    let states = focus_session_circles(count);
    if states.is_empty() {
        return;
    }
    ui.add_space(4.0);
    let fill = ui.visuals().selection.bg_fill;
    for _ in states {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(FOCUS_SESSION_CIRCLE_SIZE, FOCUS_SESSION_CIRCLE_SIZE),
            egui::Sense::hover(),
        );
        ui.painter()
            .circle_filled(rect.center(), FOCUS_SESSION_CIRCLE_SIZE / 2.0, fill);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::TodoItem;
    use crate::timer::POMODOROS_PER_LONG_BREAK;
    use eframe::egui::{self, Color32, RichText, WindowLevel};

    #[test]
    fn centered_content_has_equal_padding_and_never_goes_negative() {
        assert_eq!(centered_left_pad(330.0, 240.0), 45.0);
        assert_eq!(centered_left_pad(240.0, 240.0), 0.0);
        assert_eq!(centered_left_pad(200.0, 240.0), 0.0);
    }

    #[test]
    fn completed_todos_render_with_strikethrough() {
        let pending = TodoItem {
            text: "Buy milk".to_owned(),
            done: false,
            sessions: 0,
        };
        let done = TodoItem {
            text: "Buy milk".to_owned(),
            done: true,
            sessions: 0,
        };

        assert_eq!(todo_text(&pending), RichText::new("Buy milk"));
        assert_eq!(
            todo_text(&done),
            RichText::new("Buy milk").strikethrough().weak()
        );
        assert_eq!(todo_text(&done).text(), "Buy milk");
    }

    #[test]
    fn pin_button_label_follows_the_pin_state() {
        assert_eq!(pin_button_label(true), "Unpin window");
        assert_eq!(pin_button_label(false), "Pin on top");
    }

    #[test]
    fn pomodoro_content_is_top_aligned_with_a_small_padding() {
        assert_eq!(pomodoro_top_space(320.0), POMODORO_TOP_PADDING);
        assert_eq!(pomodoro_top_space(2.0), 2.0);
    }

    #[test]
    fn pomodoro_panel_side_is_a_square_that_fits_the_available_width() {
        assert_eq!(pomodoro_panel_side(500.0), POMODORO_PANEL_SIZE);
        assert_eq!(
            pomodoro_panel_side(300.0),
            300.0 - 2.0 * POMODORO_PANEL_MARGIN
        );
        assert_eq!(pomodoro_panel_side(10.0), 0.0);
    }

    #[test]
    fn pomodoro_section_width_includes_its_frame_margins() {
        let panel_width = POMODORO_PANEL_SIZE + 2.0 * POMODORO_PANEL_MARGIN;

        assert_eq!(pomodoro_section_width(500.0), panel_width);
        assert_eq!(pomodoro_section_width(300.0), 300.0);
    }

    #[test]
    fn progress_circles_are_small_and_fit_in_the_panel() {
        let row = progress_row_width(POMODOROS_PER_LONG_BREAK as usize, PROGRESS_CIRCLE_SIZE, 8.0);
        let previous_row = progress_row_width(POMODOROS_PER_LONG_BREAK as usize, 12.0, 8.0);
        assert!(row > 0.0 && row < previous_row);
        assert!(row < POMODORO_PANEL_SIZE / 2.0);
    }

    #[test]
    fn progress_circles_handle_an_empty_cycle() {
        assert!(progress_circles(3, 0).is_empty());
    }

    #[test]
    fn progress_circles_track_the_current_pomodoro_cycle() {
        assert_eq!(
            progress_circles(0, POMODOROS_PER_LONG_BREAK),
            vec![CircleState::Empty; POMODOROS_PER_LONG_BREAK as usize]
        );
        assert_eq!(
            progress_circles(2, POMODOROS_PER_LONG_BREAK),
            vec![
                CircleState::Filled,
                CircleState::Filled,
                CircleState::Empty,
                CircleState::Empty,
            ]
        );
        assert_eq!(
            progress_circles(POMODOROS_PER_LONG_BREAK, POMODOROS_PER_LONG_BREAK),
            vec![CircleState::Empty; POMODOROS_PER_LONG_BREAK as usize]
        );
        assert_eq!(
            progress_circles(POMODOROS_PER_LONG_BREAK + 1, POMODOROS_PER_LONG_BREAK),
            vec![
                CircleState::Filled,
                CircleState::Empty,
                CircleState::Empty,
                CircleState::Empty,
            ]
        );
    }

    #[test]
    fn start_button_has_padding_around_its_label() {
        assert_eq!(start_button_size().x, BUTTON_WIDTH + 2.0 * BUTTON_PADDING);
        assert_eq!(
            start_button_size().y,
            BUTTON_BASE_HEIGHT + 2.0 * BUTTON_PADDING
        );
        assert!(start_button_size().y > BUTTON_BASE_HEIGHT);
    }

    #[test]
    fn timer_and_circles_have_vertical_breathing_room() {
        let margin = timer_group_margin();
        assert_eq!(margin.top, 32);
        assert_eq!(margin.bottom, 32);
        assert_eq!(margin.left, 0);
        assert_eq!(margin.right, 0);
    }

    #[test]
    fn todo_input_footer_is_anchored_to_the_bottom_of_the_window() {
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(390.0, 800.0));

        let footer = todo_footer_rect(window);
        let content = todo_content_rect(window);

        assert_eq!(footer.max, window.max);
        assert_eq!(footer.height(), todo_footer_height());
        assert_eq!(content.max.y, footer.min.y);
        assert_eq!(content.min, window.min);
    }

    #[test]
    fn todo_input_has_a_drop_shadow() {
        let shadow = todo_input_shadow();

        assert!(shadow.offset[1] > 0);
        assert!(shadow.blur > 0);
        assert!(shadow.color.a() > 0);
    }

    #[test]
    fn todo_input_has_a_grey_background() {
        let color = todo_input_background();

        assert_eq!(color.r(), color.g());
        assert_eq!(color.g(), color.b());
        assert!(color.r() > 0 && color.r() < 255);
    }

    #[test]
    fn todo_input_has_an_eighty_pixel_bottom_margin() {
        assert_eq!(todo_input_bottom_margin(), 80.0);
        assert!(window_inner_size()[1] >= 680.0 + todo_input_bottom_margin());
    }

    #[test]
    fn todo_input_frame_combines_grey_fill_padding_and_shadow() {
        let frame = todo_input_frame();

        assert_eq!(frame.fill, todo_input_background());
        assert_eq!(frame.inner_margin, todo_input_margin());
        assert_eq!(frame.shadow.offset, todo_input_shadow().offset);
    }

    #[test]
    fn todo_input_has_extra_padding() {
        let margin = todo_input_margin();

        assert!(margin.left >= 10);
        assert!(margin.right >= 10);
        assert!(margin.top >= 6);
        assert!(margin.bottom >= 6);
    }

    #[test]
    fn todo_input_matches_the_pomodoro_panel_width() {
        let panel_width = POMODORO_PANEL_SIZE + 2.0 * POMODORO_PANEL_MARGIN;

        assert_eq!(todo_input_width(500.0), panel_width);
        assert_eq!(todo_input_width(300.0), 300.0);
    }

    #[test]
    fn todo_input_text_is_centered() {
        assert!(todo_input_align() == egui::Align2::CENTER_CENTER);
    }

    #[test]
    fn todo_is_submitted_only_when_enter_ends_the_input() {
        assert!(should_submit_todo(true, true));
        assert!(!should_submit_todo(false, true));
        assert!(!should_submit_todo(true, false));
    }

    #[test]
    fn todo_list_keeps_a_usable_minimum_when_space_is_tight() {
        assert_eq!(todo_list_max_height(0.0), TODO_LIST_MIN_HEIGHT);
        assert_eq!(todo_list_max_height(10.0), TODO_LIST_MIN_HEIGHT);
        assert_eq!(todo_list_max_height(120.0), 120.0);
    }

    #[test]
    fn todo_list_uses_the_space_above_the_footer() {
        let content_height = window_inner_size()[1] - todo_footer_height();

        assert_eq!(todo_list_max_height(content_height), content_height);
        assert!(content_height > TODO_LIST_MIN_HEIGHT);
    }

    #[test]
    fn window_is_tall_enough_to_show_the_todo_section() {
        let [width, height] = window_inner_size();

        assert!(width >= 340.0);
        assert!(height >= 680.0);
    }

    #[test]
    fn window_level_follows_the_pin_state() {
        assert_eq!(window_level(true), WindowLevel::AlwaysOnTop);
        assert_eq!(window_level(false), WindowLevel::Normal);
    }

    #[test]
    fn todo_row_hover_detection_covers_the_whole_row() {
        let ctx = egui::Context::default();
        ctx.set_fonts(egui::FontDefinitions::empty());
        let row =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, TODO_ROW_HEIGHT));
        let over_checkbox = egui::pos2(10.0, TODO_ROW_HEIGHT / 2.0);

        let output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(over_checkbox)],
                ..Default::default()
            },
            |ui| {
                let mut checked = false;
                ui.checkbox(&mut checked, "Buy milk");
                assert!(todo_row_hovered(ui, row));
                assert!(!todo_row_hovered(
                    ui,
                    row.translate(egui::vec2(0.0, TODO_ROW_HEIGHT * 2.0))
                ));
            },
        );
        output.drop_without_applying_deltas();
    }

    #[test]
    fn todo_section_container_is_centered_and_content_is_left_aligned() {
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(390.0, 800.0));

        egui::__run_test_ui(|ui| {
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(window)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            let width = todo_section_width(content.available_width());
            let mut container = egui::Rect::NOTHING;
            let mut label_x = Vec::new();

            content.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    container = ui.max_rect();
                    label_x.push(ui.label("TO-DO LIST").rect.min.x);
                    label_x.push(ui.label("Nothing to do yet").rect.min.x);
                },
            );

            assert_eq!(container.width(), width);
            assert!((container.center().x - window.center().x).abs() < 1.0);
            assert!(label_x.iter().all(|x| (*x - container.min.x).abs() < 1.0));
        });
    }

    #[test]
    fn todo_list_scroll_area_reaches_the_container_edge() {
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(390.0, 800.0));

        egui::__run_test_ui(|ui| {
            let mut content = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(window)
                    .layout(egui::Layout::top_down(egui::Align::Center)),
            );
            let width = todo_section_width(content.available_width());
            let mut container_width = 0.0;
            let mut final_scroll_width = 0.0;

            content.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    container_width = ui.max_rect().width();
                    let output = todo_list_scroll_area(300.0).show(ui, |ui| {
                        for text in ["Buy milk", "Walk the dog", "Write report"] {
                            let mut checked = false;
                            ui.checkbox(&mut checked, text);
                        }
                    });

                    // A vertical ScrollArea shrinks to content width when auto_shrink.x is set.
                    let auto_shrink = todo_list_auto_shrink();
                    final_scroll_width = if auto_shrink.x {
                        output.content_size.x
                    } else {
                        output.inner_rect.width()
                    };
                },
            );

            assert!(
                (final_scroll_width - container_width).abs() < 1.0,
                "scroll area is {}px wide, container is {}px wide",
                final_scroll_width,
                container_width
            );
        });
    }

    #[test]
    fn todo_list_shows_every_item_when_they_fit_in_the_available_space() {
        egui::__run_test_ui(|ui| {
            let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 300.0));
            let mut list = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(area)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );

            let output = todo_list_scroll_area(list.available_height()).show(&mut list, |ui| {
                for number in 0..8 {
                    let item = TodoItem {
                        text: format!("Task {number}"),
                        done: false,
                        sessions: 0,
                    };
                    todo_row(ui, &item, false);
                }
            });

            assert!(
                output.inner_rect.height() >= output.content_size.y,
                "list viewport is {}px but content is {}px",
                output.inner_rect.height(),
                output.content_size.y
            );
        });
    }

    #[test]
    fn todo_list_still_scrolls_when_items_exceed_the_available_space() {
        egui::__run_test_ui(|ui| {
            let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(300.0, 300.0));
            let mut list = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(area)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );

            let output = todo_list_scroll_area(list.available_height()).show(&mut list, |ui| {
                for number in 0..20 {
                    let item = TodoItem {
                        text: format!("Task {number}"),
                        done: false,
                        sessions: 0,
                    };
                    todo_row(ui, &item, false);
                }
            });

            assert!(output.content_size.y > output.inner_rect.height());
            assert!(output.inner_rect.height() <= area.height());
        });
    }

    #[test]
    fn todo_row_background_appears_only_on_hover() {
        let idle = todo_row_background(false);
        let hovered = todo_row_background(true);

        assert_eq!(idle, Color32::TRANSPARENT);
        assert_ne!(hovered, idle);
        assert!(hovered.a() > 0);
        assert!(hovered.r() < 100);
    }

    #[test]
    fn todo_section_fits_narrow_windows_and_stays_centered() {
        assert_eq!(todo_section_width(10.0), 10.0);
        assert_eq!(todo_section_width(300.0), 300.0);

        let available = 390.0;
        let width = todo_section_width(available);
        let left = centered_left_pad(available, width);
        assert_eq!(left, available - left - width);
    }

    #[test]
    fn todo_section_width_matches_the_pomodoro_section() {
        let panel_width = POMODORO_PANEL_SIZE + 2.0 * POMODORO_PANEL_MARGIN;

        assert_eq!(todo_section_width(500.0), panel_width);
        assert_eq!(todo_section_width(390.0), panel_width);
        assert_eq!(todo_input_width(500.0), todo_section_width(500.0));
    }

    #[test]
    fn focus_button_label_follows_the_focus_state() {
        assert_eq!(focus_button_label(true), "◉");
        assert_eq!(focus_button_label(false), "◎");
    }

    #[test]
    fn focus_session_circles_match_the_completed_sessions() {
        assert!(focus_session_circles(0).is_empty());
        assert_eq!(focus_session_circles(3), vec![CircleState::Filled; 3]);
    }

    #[test]
    fn todo_row_renders_focus_and_session_circles() {
        egui::__run_test_ui(|ui| {
            let focused_item = TodoItem {
                text: "Write report".to_owned(),
                done: false,
                sessions: 3,
            };

            let response = todo_row(ui, &focused_item, true);

            assert!(!response.toggled);
            assert!(!response.removed);
            assert!(!response.focus_toggled);
        });
    }
}
