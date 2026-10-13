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
use std::time::{Duration, Instant};
mod data;
mod pages;
mod sys;
mod tabs;
mod theme;
mod tools;
use tabs::TabManager;

use crate::data::options::{CLEANING_OPTIONS, TWEAKS_OPTIONS};
use crate::tools::{cleaning, tweaks};

/// Key hints shown at the bottom-right of the window. "magenta" refers to the
/// highlighted rows: they need administrator rights.
const KEY_HINTS: &str = "↑↓ move · Enter run · ←→ tabs · q quit · magenta = admin";
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

/// Spinner frames for the running indicator, advanced by the redraw loop.
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// What the status bar shows: either the running indicator of an in-flight
/// job or the last action's result (with its error state, so failures can be
/// coloured without parsing the text).
enum StatusLine<'a> {
    Running(&'a str),
    Message { text: &'a str, is_error: bool },
}

/// A tab action running on a worker thread, so the UI keeps animating while
/// slow work (activation, appx removal, downloads) completes.
struct Job {
    /// Row label shown next to the spinner.
    label: String,
    /// When the job started, drives the spinner frame and the elapsed counter.
    started: Instant,
    /// The running action's result, ready when `is_finished()` returns true.
    handle: std::thread::JoinHandle<Result<String, String>>,
    /// Which tab's status snapshot to refresh when the job finishes.
    tab: usize,
    /// True only for the activation action, whose slow WMI check is refreshed
    /// separately when the job finishes.
    refresh_activation: bool,
}

/// The event loop. Draws a frame, polls events for a short moment, then loops
/// again — which keeps the running-indicator animating while a job works on
/// its own thread. Returns when the user quits.
fn run(
    terminal: &mut DefaultTerminal,
    tab_manager: &mut TabManager,
    table_state: &mut TableState,
) -> Result<()> {
    // Cached UI state, refreshed only when it can actually change.
    let mut cleaning_status = cleaning::CleaningStatus::new();
    let mut tweaks_status = tweaks::TweaksStatus::new();
    // Result of the last action, shown in the status bar.
    let mut message = String::new();
    // Whether `message` is an error (drives the red styling).
    let mut message_is_error = false;
    // A process's elevation never changes after launch, so check it once.
    let is_admin = sys::is_admin();
    // The action currently running on a worker thread, if any.
    let mut job: Option<Job> = None;
    // Set after a first quit press while a job is running.
    let mut quit_armed = false;

    loop {
        // Compose the running indicator; redrawing advances the spinner.
        // (The nested ifs below are deliberately not collapsed into one
        // let-chain: two plain blocks are easier to read for new Rust
        // programmers.)
        #[allow(clippy::collapsible_if)]
        {
            let running = job.as_ref().map(|job| {
                let ticks = job.started.elapsed().as_millis() as usize / 125;
                let frame = SPINNER[ticks % SPINNER.len()];
                format!(
                    "{frame} Running: {}… ({}s)",
                    job.label,
                    job.started.elapsed().as_secs()
                )
            });
            terminal.draw(|frame| {
                let status = if let Some(text) = running.as_deref() {
                    StatusLine::Running(text)
                } else {
                    StatusLine::Message {
                        text: &message,
                        is_error: message_is_error,
                    }
                };
                draw(
                    frame,
                    tab_manager,
                    table_state,
                    &cleaning_status,
                    &tweaks_status,
                    is_admin,
                    status,
                )
            })?;

            // Poll events with a timeout so the spinner keeps animating.
            if event::poll(Duration::from_millis(125))? {
                if let Event::Key(key) = event::read()? {
                    // On Windows crossterm reports both press and release, so
                    // ignore everything that is not a press.
                    if key.kind != event::KeyEventKind::Press {
                        continue;
                    }

                    let mut quit_requested = false;
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => quit_requested = true,
                        // Ctrl+C is the universal abort key; raw mode delivers
                        // it as a plain character, so handle it explicitly.
                        KeyCode::Char('c')
                            if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                        {
                            quit_requested = true
                        }

                        // table navigation
                        KeyCode::Up | KeyCode::Char('k') => {
                            let selected = table_state.selected().unwrap_or(0);
                            if selected > 0 {
                                table_state.select(Some(selected - 1));
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let selected = table_state.selected().unwrap_or(0);
                            if selected + 1 < item_count(tab_manager.get_selection()) {
                                table_state.select(Some(selected + 1));
                            }
                        }
                        KeyCode::PageUp => {
                            let selected = table_state.selected().unwrap_or(0);
                            table_state.select(Some(selected.saturating_sub(10)));
                        }
                        KeyCode::PageDown => {
                            let selected = table_state.selected().unwrap_or(0);
                            let last = item_count(tab_manager.get_selection())
                                .saturating_sub(1);
                            table_state.select(Some((selected + 10).min(last)));
                        }
                        KeyCode::Home => table_state.select(Some(0)),
                        KeyCode::End => {
                            let last = item_count(tab_manager.get_selection())
                                .saturating_sub(1);
                            table_state.select(Some(last));
                        }

                        // run the selected action on a worker thread
                        KeyCode::Enter => {
                            if job.is_some() {
                                message = "a task is still running — wait for it to finish"
                                    .to_string();
                                message_is_error = true;
                            } else if let Some(index) = table_state.selected() {
                                let selected = match tab_manager.get_selection() {
                                    0 => pages::cleaning::action(index),
                                    1 => pages::tweaks::action(index),
                                    _ => None,
                                };
                                if let Some((action, label, needs_admin)) = selected {
                                    if needs_admin && !is_admin {
                                        // Admin-only tweaks fail unelevated;
                                        // say so instead of starting a doomed
                                        // worker.
                                        message = "this tweak needs administrator rights — restart PatchKit as administrator"
                                            .to_string();
                                        message_is_error = true;
                                    } else {
                                        // The activation check is slow, so it
                                        // is only refreshed after the
                                        // activation action itself.
                                        let refresh_activation =
                                            tab_manager.get_selection() == 1
                                                && label == "Windows Activation";
                                        quit_armed = false;
                                        message_is_error = false;
                                        job = Some(Job {
                                            label: label.to_string(),
                                            started: Instant::now(),
                                            handle: std::thread::spawn(action),
                                            tab: tab_manager.get_selection(),
                                            refresh_activation,
                                        });
                                    }
                                }
                            }
                        }

                        // tab navigation (start each tab at the first row)
                        KeyCode::Left | KeyCode::Char('h') => {
                            tab_manager.prev_tab();
                            table_state.select(Some(0));
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            tab_manager.next_tab();
                            table_state.select(Some(0));
                        }

                        // refresh the cheap status values on request
                        KeyCode::Char('r') => {
                            cleaning_status.refresh();
                            tweaks_status.refresh();
                            message = "status refreshed".to_string();
                            message_is_error = false;
                        }

                        _ => {}
                    }

                    if quit_requested {
                        // Quitting kills a still-running worker mid-task, so
                        // require a second press to confirm.
                        if job.is_some() && !quit_armed {
                            quit_armed = true;
                            message = "a task is still running — press q again to quit anyway"
                                .to_string();
                            message_is_error = true;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Collect finished worker threads and show their result.
        if job.as_ref().is_some_and(|job| job.handle.is_finished()) {
            let job = job.take().expect("job just finished");
            let result = job
                .handle
                .join()
                .unwrap_or_else(|_| Err("worker thread crashed".to_string()));
            message_is_error = result.is_err();
            message = result_message(result);
            quit_armed = false;
            match job.tab {
                0 => cleaning_status.refresh(),
                1 => {
                    tweaks_status.refresh();
                    if job.refresh_activation {
                        tweaks_status.refresh_activation();
                    }
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
    tab_manager: &TabManager,
    table_state: &mut TableState,
    cleaning_status: &cleaning::CleaningStatus,
    tweaks_status: &tweaks::TweaksStatus,
    is_admin: bool,
    status: StatusLine,
) {
    // Leave a gutter so content is not flush against the terminal edges.
    let area = frame.area().inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    // The ASCII banner only fits when the terminal is wide enough for it and
    // the tagline, and tall enough to leave the rest of the UI a row or two.
    // Otherwise fall back to a one-line text title carrying everything.
    let banner_width = BANNER
        .lines()
        .map(|line| line.trim_end().chars().count())
        .max()
        .unwrap_or(0);
    let banner_height = BANNER.lines().count() as u16;
    let role = if is_admin { "Administrator" } else { "User" };
    let wide_tagline = format!("{TAGLINE} · {VERSION}");
    let narrow_tagline = format!("{wide_tagline} · {role}");
    let show_banner = area.width as usize >= banner_width + wide_tagline.chars().count()
        && area.height >= banner_height + 4;
    let tagline = if show_banner {
        wide_tagline
    } else {
        narrow_tagline
    };
    let title_height = if show_banner {
        banner_height
    } else {
        1
    };

    // Rows: title, separator, tab bar, the table, the status bar.
    let [
        title_area,
        rule_area,
        tab_area,
        body_area,
        status_area,
    ] = Layout::vertical([
        Constraint::Length(title_height),
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

    // Tagline at the top-right of the title, elevation at the bottom-right.
    // On narrow terminals the bottom slot has zero height, so everything is
    // merged into the single tagline line instead.
    let [tagline_top, _fill, tagline_bottom] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(tagline_area);

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
        .render(tagline_top, frame.buffer_mut());
    Paragraph::new(role)
        .alignment(Alignment::Right)
        .style(theme::dim())
        .render(tagline_bottom, frame.buffer_mut());

    // Separator bar under the title.
    Paragraph::new("─".repeat(rule_area.width as usize))
        .style(theme::dim())
        .render(rule_area, frame.buffer_mut());

    // Tab bar, with the active tab in the accent colour.
    Tabs::new(
        tab_manager
            .titles()
            .iter()
            .map(|title| title.as_str())
            .collect::<Vec<_>>(),
    )
    .style(theme::tab())
    .highlight_style(theme::tab_active())
    .padding("", "")
    .divider("   ")
    .select(tab_manager.get_selection())
    .render(tab_area, frame.buffer_mut());

    // Body: the table of the currently selected tab.
    match tab_manager.get_selection() {
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

    // While a job runs, the spinner text replaces the last message; errors
    // still stand out in red afterwards.
    let (text, is_error) = match status {
        StatusLine::Running(text) => (text, false),
        StatusLine::Message { text, is_error } => (text, is_error),
    };
    let style = if is_error { theme::error() } else { theme::dim() };
    Paragraph::new(text)
        .style(style)
        .render(message_area, frame.buffer_mut());
    Paragraph::new(KEY_HINTS)
        .alignment(Alignment::Right)
        .style(theme::dim())
        .render(hints_area, frame.buffer_mut());
}
