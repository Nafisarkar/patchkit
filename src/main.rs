//! PatchKit — a small Windows cleanup and tweaks TUI.
//!
//! This is the entry point. It builds the tab list, runs the event loop
//! (draw a frame, wait for a key, handle it), and lays out the window.

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Margin},
    widgets::{Paragraph, TableState, Tabs, Widget},
};
mod data;
mod pages;
mod sys;
mod tabs;
mod theme;
mod tools;
use tabs::TabManager;

use crate::data::options::{CLEANING_OPTIONS, TWEAKS_OPTIONS};
use crate::tools::{cleaning, tweaks};

/// Key hints shown at the bottom-right of the window.
const KEY_HINTS: &str = "↑↓ move   ⏎ run   ←→ tabs   q quit";
/// Short tagline shown at the top-right.
const TAGLINE: &str = "cleanup & tweaks toolbox";
/// App version, taken from Cargo.toml at compile time.
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));
/// ASCII art banner shown in the title bar on wide-enough terminals.
const BANNER: &str = "\
██████╗  █████╗ ████████╗ ██████╗██╗  ██╗██╗  ██╗██╗████████╗
██╔══██╗██╔══██╗╚══██╔══╝██╔════╝██║  ██║██║ ██╔╝██║╚══██╔══╝
██████╔╝███████║   ██║   ██║     ███████║█████╔╝ ██║   ██║   
██╔═══╝ ██╔══██║   ██║   ██║     ██╔══██║██╔═██╗ ██║   ██║   
██║     ██║  ██║   ██║   ╚██████╗██║  ██║██║  ██╗██║   ██║   
╚═╝     ╚═╝  ╚═╝   ╚═╝    ╚═════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝   ╚═╝";

/// Entry point: register the tabs, start the terminal, run the app, then make
/// sure the terminal is restored even if the app exits early.
fn main() -> Result<()> {
    let mut manager = TabManager::new();
    manager.add_tab("Cleaning");
    manager.add_tab("Tweaks");

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut manager, &mut table_state);
    ratatui::restore();
    result
}

/// The event loop. Draws a frame, blocks until a key is pressed, handles that
/// key, then loops again. Returns when the user quits.
fn run(
    terminal: &mut DefaultTerminal,
    tabmanager: &mut TabManager,
    table_state: &mut TableState,
) -> Result<()> {
    // Cached UI state, refreshed only when it can actually change.
    let mut cleaning_status = cleaning::CleaningStatus::new();
    let mut tweaks_status = tweaks::TweaksStatus::new();
    // Result of the last action, shown in the status bar.
    let mut message = String::new();

    loop {
        // draw
        terminal.draw(|frame| {
            draw(
                frame,
                tabmanager,
                table_state,
                &cleaning_status,
                &tweaks_status,
                &message,
            )
        })?;

        // event handler
        if let Event::Key(key) = event::read()? {
            // On Windows crossterm reports both press and release, so ignore
            // everything that is not a press to avoid handling a key twice.
            if key.kind != event::KeyEventKind::Press {
                continue;
            }

            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,

                // table navigation
                KeyCode::Up | KeyCode::Char('k') => {
                    let selected = table_state.selected().unwrap_or(0);
                    if selected > 0 {
                        table_state.select(Some(selected - 1));
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let selected = table_state.selected().unwrap_or(0);
                    if selected + 1 < item_count(tabmanager.get_selection()) {
                        table_state.select(Some(selected + 1));
                    }
                }

                // run the selected action
                KeyCode::Enter => {
                    if let Some(index) = table_state.selected() {
                        match tabmanager.get_selection() {
                            0 => {
                                message = result_message(pages::cleaning::activate(
                                    index,
                                    &mut cleaning_status,
                                ));
                            }
                            1 => {
                                message = result_message(pages::tweaks::activate(
                                    index,
                                    &mut tweaks_status,
                                ));
                            }
                            _ => {}
                        }
                    }
                }

                // tab navigation (start each tab at the first row)
                KeyCode::Left | KeyCode::Char('h') => {
                    tabmanager.prev_tab();
                    table_state.select(Some(0));
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    tabmanager.next_tab();
                    table_state.select(Some(0));
                }

                _ => {}
            }
        }
    }

    Ok(())
}

/// Number of selectable rows on the given tab.
fn item_count(tab: usize) -> usize {
    match tab {
        0 => CLEANING_OPTIONS.len(),
        _ => TWEAKS_OPTIONS.len(),
    }
}

/// Turn an action's result into one line for the status bar: the success text,
/// or the error prefixed with "Error:".
fn result_message(result: Result<String, String>) -> String {
    match result {
        Ok(text) => text,
        Err(text) => format!("Error: {text}"),
    }
}

/// Draw one frame: title bar, separator, tab bar, the active page's table, and
/// the status bar.
fn draw(
    frame: &mut Frame,
    tabmanager: &TabManager,
    table_state: &mut TableState,
    cleaning_status: &cleaning::CleaningStatus,
    tweaks_status: &tweaks::TweaksStatus,
    message: &str,
) {
    // Leave a gutter so content is not flush against the terminal edges.
    let area = frame.area().inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // The ASCII banner only fits when the terminal is wide enough for it and
    // the tagline; otherwise fall back to a one-line text title.
    let banner_width = BANNER
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    let long_tagline = format!("{TAGLINE} · {VERSION}");
    let show_banner = area.width as usize >= banner_width + long_tagline.chars().count();
    let tagline = if show_banner {
        long_tagline
    } else {
        TAGLINE.to_string()
    };
    let title_height = if show_banner {
        BANNER.lines().count() as u16
    } else {
        1
    };

    // Rows: title, separator, tab bar, a spacer, the table, the status bar.
    let [
        title_area,
        rule_area,
        tab_area,
        _gap,
        body_area,
        status_area,
    ] = Layout::vertical([
        Constraint::Length(title_height),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Title bar: the banner on the left, a tagline on the right.
    let [name_area, tagline_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(tagline.chars().count() as u16),
    ])
    .areas(title_area);
    let title = if show_banner {
        BANNER.to_string()
    } else {
        format!("PatchKit {VERSION}")
    };
    Paragraph::new(title)
        .style(theme::title())
        .render(name_area, frame.buffer_mut());
    Paragraph::new(tagline)
        .alignment(Alignment::Right)
        .style(theme::dim())
        .render(tagline_area, frame.buffer_mut());

    // Separator bar under the title.
    Paragraph::new("─".repeat(rule_area.width as usize))
        .style(theme::dim())
        .render(rule_area, frame.buffer_mut());

    // Tab bar, with the active tab in the accent colour.
    Tabs::new(
        tabmanager
            .get_tabs()
            .iter()
            .map(|tab| tab.title.as_str())
            .collect::<Vec<_>>(),
    )
    .style(theme::tab())
    .highlight_style(theme::tab_active())
    .padding("", "")
    .divider("   ")
    .select(tabmanager.get_selection())
    .render(tab_area, frame.buffer_mut());

    // Body: the table of the currently selected tab.
    match tabmanager.get_selection() {
        0 => pages::cleaning::draw(frame, body_area, table_state, cleaning_status),
        1 => pages::tweaks::draw(frame, body_area, table_state, tweaks_status),
        _ => {}
    }

    // Status bar: the last action's message on the left, key hints on the
    // right. Two extra columns keep the message from running into the hints.
    let [message_area, hints_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(KEY_HINTS.chars().count() as u16 + 2),
    ])
    .areas(status_area);

    Paragraph::new(message).render(message_area, frame.buffer_mut());
    Paragraph::new(KEY_HINTS)
        .alignment(Alignment::Right)
        .style(theme::dim())
        .render(hints_area, frame.buffer_mut());
}
