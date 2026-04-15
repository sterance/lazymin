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
    pub competitors: Option<Rect>,
    pub terminal: Rect,
    pub log: Rect,
}

pub fn left_rail_columns(compact_left_rail: bool) -> u16 {
    if compact_left_rail {
        18
    } else {
        30
    }
}

pub fn compute(area: Rect, market_unlocked: bool, compact_left_rail: bool) -> AppAreas {
    compute_full(area, market_unlocked, false, compact_left_rail)
}

pub fn compute_full(
    area: Rect,
    market_unlocked: bool,
    competitors_active: bool,
    compact_left_rail: bool,
) -> AppAreas {
    let area = content_area_with_left_padding(area);
    let left_w = left_rail_columns(compact_left_rail);
    let vertical = if compact_left_rail {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(8),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(8),
            ])
            .split(area)
    };

    let (header_idx, main_idx, log_idx) = if compact_left_rail {
        (1, 3, 4)
    } else {
        (0, 2, 3)
    };

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_w), Constraint::Min(1)])
        .split(vertical[main_idx]);

    let (resources, market, competitors) = if market_unlocked && competitors_active {
        let resources_min = if compact_left_rail { 23 } else { 8 };
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(resources_min),
                Constraint::Length(6),
                Constraint::Length(7),
            ])
            .split(top[0]);
        (left[0], Some(left[1]), Some(left[2]))
    } else if market_unlocked {
        let resources_min = if compact_left_rail { 23 } else { 8 };
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(resources_min), Constraint::Length(6)])
            .split(top[0]);
        (left[0], Some(left[1]), None)
    } else {
        (top[0], None, None)
    };

    AppAreas {
        header: vertical[header_idx],
        resources,
        market,
        competitors,
        terminal: top[1],
        log: vertical[log_idx],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_rail_width_matches_compact_flag() {
        let area = Rect::new(0, 0, 80, 40);
        let wide = compute(area, false, false);
        let compact = compute(area, false, true);
        assert_eq!(wide.resources.width, 30);
        assert_eq!(compact.resources.width, 18);
    }
}
