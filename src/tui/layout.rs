use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn chunks(area: Rect, input_height: u16) -> Vec<Rect> {
    let input_h = input_height.clamp(3, 10);
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(input_h),
            Constraint::Length(1),
        ])
        .split(area)
        .to_vec()
}
