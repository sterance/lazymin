use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppAreas {
    pub header: Rect,
    pub resources: Rect,
    pub terminal: Rect,
    pub log: Rect,
}

pub fn compute(area: Rect) -> AppAreas {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(8),
        ])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(1)])
        .split(vertical[2]);

    AppAreas {
        header: vertical[0],
        resources: top[0],
        terminal: top[1],
        log: vertical[3],
    }
}
