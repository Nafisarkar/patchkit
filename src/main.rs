//! PatchKit — a small Windows cleanup and tweaks TUI.
//!
//! This is the entry point. It builds the tab list, runs the event loop
//! (draw a frame, wait for a key, handle it), and lays out the window: a
//! title bar, a tab bar, the active page's table, and a one-line status bar.

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Margin},
    style::{Color, Modifier, Style},
    widgets::{Paragraph, TableState, Tabs, Widget},
};
mod data;
mod pages;
mod sys;
mod tabs;
mod tools;
use tabs::TabManager;

use crate::data::options::{CLEANING_OPTIONS, TWEAKS_OPTIONS};
use crate::tools::{cleaning, tweaks};

/// Key hints shown at the bottom-right of the window.
const KEY_HINTS: &str = "↑↓ move   ⏎ run   ←→ tabs   q quit";

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

/// Draw one frame: title, tab bar, the active page's table, and a status line.
fn draw(
    frame: &mut Frame,
    tabmanager: &TabManager,
    table_state: &mut TableState,
    cleaning_status: &cleaning::CleaningStatus,
    tweaks_status: &tweaks::TweaksStatus,
    message: &str,
) {
    // Keep a one-column gutter on each side; the whole UI is borderless.
    let area = frame.area().inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    // Five compact rows: title, separator, tab bar, table, status/hints.
    let [header, rule, tabbar, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Title.
    Paragraph::new("PatchKit")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .render(header, frame.buffer_mut());

    // Separator bar under the title.
    Paragraph::new("─".repeat(rule.width as usize))
        .style(Style::default().fg(Color::DarkGray))
        .render(rule, frame.buffer_mut());

    // Tab bar, with the active tab in the accent colour.
    Tabs::new(
        tabmanager
            .get_tabs()
            .iter()
            .map(|tab| tab.title.as_str())
            .collect::<Vec<_>>(),
    )
    .style(Style::default().fg(Color::DarkGray))
    .highlight_style(
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    // No padding, so the first tab lines up with the title and the table.
    .padding("", "")
    .divider("   ")
    .select(tabmanager.get_selection())
    .render(tabbar, frame.buffer_mut());

    // Body: the table of the currently selected tab.
    match tabmanager.get_selection() {
        0 => pages::cleaning::draw(frame, body, table_state, cleaning_status),
        1 => pages::tweaks::draw(frame, body, table_state, tweaks_status),
        _ => {}
    }

    // Status line: the last action's message on the left, key hints on the right.
    let [message_area, hints_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(KEY_HINTS.chars().count() as u16),
    ])
    .areas(footer);

    Paragraph::new(message).render(message_area, frame.buffer_mut());
    Paragraph::new(KEY_HINTS)
        .style(Style::default().fg(Color::DarkGray))
        .render(hints_area, frame.buffer_mut());
}
