use ratatui::layout::{Constraint, Direction, Layout, Rect};

fn content_area_with_left_padding(area: Rect) -> Rect {
    if area.width <= 1 {
        return area;
    }
    Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width - 1,
        height: area.height,
    }
}

pub struct AppAreas {
    pub header: Rect,
    pub resources: Rect,
    pub market: Option<Rect>,
    pub terminal: Rect,
    pub log: Rect,
}

pub fn compute(area: Rect, market_unlocked: bool) -> AppAreas {
    let area = content_area_with_left_padding(area);
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

    let (resources, market) = if market_unlocked {
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(7)])
            .split(top[0]);
        (left[0], Some(left[1]))
    } else {
        (top[0], None)
    };

    AppAreas {
        header: vertical[0],
        resources,
        market,
        terminal: top[1],
        log: vertical[3],
    }
}
