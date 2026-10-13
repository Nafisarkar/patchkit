//! The Store tab: download and install the apps listed in `STORE_CATEGORIES`,
//! grouped under category headers.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Cell, Row, TableState},
};

use crate::data::options::{Job, STORE_CATEGORIES, StoreApp};
use crate::theme;
use crate::tools::downloader::{self, StoreStatus};

/// A Store row: a category header, or one app.
enum StoreRow {
    Header(&'static str),
    App(&'static StoreApp),
}

/// The rows in display order, with a header before each category's apps.
fn rows() -> Vec<StoreRow> {
    STORE_CATEGORIES
        .iter()
        .flat_map(|category| {
            std::iter::once(StoreRow::Header(category.title))
                .chain(category.apps.iter().map(StoreRow::App))
        })
        .collect()
}

/// Total number of rows, category headers included.
pub fn row_count() -> usize {
    STORE_CATEGORIES
        .iter()
        .map(|category| category.apps.len() + 1)
        .sum()
}

/// True when `row` is a category header, which the cursor skips.
pub fn is_header(row: usize) -> bool {
    matches!(rows().get(row), Some(StoreRow::Header(_)))
}

/// Render the Store table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &StoreStatus) {
    let mut app_index = 0;
    let rows = rows()
        .into_iter()
        .map(|row| match row {
            StoreRow::Header(title) => header_row(title),
            StoreRow::App(app) => {
                let value = status.value(app_index);
                app_index += 1;
                super::option_row(app.name, app.description, false, value)
            }
        })
        .collect::<Vec<_>>();

    super::action_table(frame, area, state, rows);
}

/// The action behind row `row`, if it holds an app; `main.rs` runs it on a
/// worker thread so the UI keeps animating.
pub fn action(row: usize) -> Option<(Job, &'static str, bool)> {
    let app = match rows().get(row)? {
        StoreRow::App(app) => *app,
        StoreRow::Header(_) => return None,
    };
    Some((Box::new(move || downloader::install(app)), app.name, false))
}

/// A category header row, spanning the table in the heading style.
fn header_row(title: &str) -> Row<'static> {
    Row::new(vec![
        Cell::from(title.to_string()),
        Cell::from(""),
        Cell::from(""),
    ])
    .style(theme::heading())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_interleave_headers_and_apps() {
        let apps: usize = STORE_CATEGORIES.iter().map(|c| c.apps.len()).sum();
        let categories = STORE_CATEGORIES.len();
        assert_eq!(row_count(), apps + categories);
        // The first row is a header; the row after it is the first app.
        assert!(is_header(0));
        assert!(!is_header(1));
        assert_eq!(action(1).map(|(_, name, _)| name), Some("VSCode"));
        // A header row has no action.
        assert!(action(0).is_none());
    }
}
