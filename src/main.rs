//! PatchKit — a small Windows cleanup and tweaks TUI.
//!
//! This is the entry point. It builds the tab list, runs the event loop
//! (draw a frame, wait for a key, handle it), and lays out the window.

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Margin, Position, Rect},
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

use crate::data::options::{CLEANING_OPTIONS, STORE_CATEGORIES, TWEAKS_OPTIONS};
use crate::tools::{cleaning, downloader, tweaks};

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

/// Divider drawn between tab titles (its width drives tab click hit-testing).
const TAB_DIVIDER: &str = "   ";

/// Screen regions from the last drawn frame, used to turn mouse coordinates
/// into a clicked tab or table row.
#[derive(Default, Clone, Copy)]
struct HitAreas {
    tab: Rect,
    list: Rect,
}

/// The cached per-tab status snapshots handed to `draw`, grouped so the draw
/// signature stays within clippy's argument limit.
struct Statuses<'a> {
    cleaning: &'a cleaning::CleaningStatus,
    tweaks: &'a tweaks::TweaksStatus,
    store: &'a downloader::StoreStatus,
}

/// Turns mouse reporting back off on every exit path, including a panic during
/// `run`, so the terminal is never left with mouse capture still on.
struct MouseGuard;

impl Drop for MouseGuard {
    fn drop(&mut self) {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    }
}

/// Entry point: register the tabs, start the terminal, run the app, then make
/// sure the terminal is restored even if the app exits early.
fn main() -> Result<()> {
    let mut manager = TabManager::new();
    manager.add_tab("Cleaning");
    manager.add_tab("Tweaks");
    manager.add_tab("Store");

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    color_eyre::install()?;
    let mut terminal = ratatui::init();
    // Turn on mouse reporting so clicks and wheel scrolling reach the event
    // loop instead of being swallowed by the terminal. Best-effort: a terminal
    // that refuses it must not stop the app or skip `restore()` below.
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    // The guard turns mouse reporting off again however `run` ends.
    let _mouse_guard = MouseGuard;
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
    let mut cleaning_status = cleaning::CleaningStatus::new();
    let mut tweaks_status = tweaks::TweaksStatus::new();
    let mut store_status = downloader::StoreStatus::new(&STORE_CATEGORIES);
    let mut message = String::new();
    let mut message_is_error = false;
    // A process's elevation never changes after launch, so check it once.
    let is_admin = sys::is_admin();
    let mut job: Option<Job> = None;
    let mut quit_armed = false;
    let mut hits = HitAreas::default();

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
                hits = draw(
                    frame,
                    tab_manager,
                    table_state,
                    Statuses {
                        cleaning: &cleaning_status,
                        tweaks: &tweaks_status,
                        store: &store_status,
                    },
                    is_admin,
                    status,
                );
            })?;

            // Poll with a timeout so the spinner keeps animating.
            if event::poll(Duration::from_millis(125))? {
                // Declared before the drain loop so a quit key seen anywhere in
                // the batch is honoured once the queue is empty.
                let mut quit_requested = false;
                // Drain every event already queued before redrawing, so a fast
                // wheel scroll or a burst of keys is not throttled to one event
                // per frame; the 125ms wait above only paces the idle case.
                while event::poll(Duration::ZERO)? {
                    match event::read()? {
                        Event::Key(key) => {
                            // On Windows crossterm reports both press and
                            // release, so ignore everything that is not a press.
                            if key.kind != event::KeyEventKind::Press {
                                continue;
                            }
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => quit_requested = true,
                                // Ctrl+C is the universal abort key; raw mode
                                // delivers it as a plain character, so handle it
                                // explicitly.
                                KeyCode::Char('c')
                                    if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                                {
                                    quit_requested = true
                                }

                                KeyCode::Up | KeyCode::Char('k') => {
                                    step_selection(tab_manager.get_selection(), table_state, -1);
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    step_selection(tab_manager.get_selection(), table_state, 1);
                                }
                                KeyCode::PageUp => {
                                    step_selection(tab_manager.get_selection(), table_state, -10);
                                }
                                KeyCode::PageDown => {
                                    step_selection(tab_manager.get_selection(), table_state, 10);
                                }
                                KeyCode::Home => {
                                    let first = first_row(tab_manager.get_selection());
                                    table_state.select(Some(first));
                                }
                                KeyCode::End => {
                                    let last = last_row(tab_manager.get_selection());
                                    table_state.select(Some(last));
                                }

                                KeyCode::Enter => {
                                    if job.is_some() {
                                        message = "a task is still running — wait for it to finish"
                                            .to_string();
                                        message_is_error = true;
                                    } else if let Some(index) = table_state.selected() {
                                        let selected = match tab_manager.get_selection() {
                                            0 => pages::cleaning::action(index),
                                            1 => pages::tweaks::action(index),
                                            _ => pages::store::action(index),
                                        };
                                        if let Some((action, label, needs_admin)) = selected {
                                            if needs_admin && !is_admin {
                                                // Admin-only tweaks fail
                                                // unelevated; say so instead of
                                                // starting a doomed worker.
                                                message = "this tweak needs administrator rights — restart PatchKit as administrator"
                                                    .to_string();
                                                message_is_error = true;
                                            } else {
                                                // The activation check is slow,
                                                // so it is only refreshed after
                                                // the activation action itself.
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

                                // Start each tab at its first selectable row.
                                KeyCode::Left | KeyCode::Char('h') => {
                                    tab_manager.prev_tab();
                                    let first = first_row(tab_manager.get_selection());
                                    table_state.select(Some(first));
                                }
                                KeyCode::Right | KeyCode::Char('l') => {
                                    tab_manager.next_tab();
                                    let first = first_row(tab_manager.get_selection());
                                    table_state.select(Some(first));
                                }

                                KeyCode::Char('r') => {
                                    cleaning_status.refresh();
                                    tweaks_status.refresh();
                                    message = "status refreshed".to_string();
                                    message_is_error = false;
                                }

                                _ => {}
                            }
                        }
                        Event::Mouse(mouse) => {
                            handle_mouse(&mouse, &hits, tab_manager, table_state)
                        }
                        _ => {}
                    }

                    if quit_requested {
                        break;
                    }
                }

                if quit_requested {
                    // Quitting kills a still-running worker mid-task, so
                    // require a second press to confirm.
                    if job.is_some() && !quit_armed {
                        quit_armed = true;
                        message =
                            "a task is still running — press q again to quit anyway".to_string();
                        message_is_error = true;
                    } else {
                        break;
                    }
                }
            }
        }

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
                2 => store_status.refresh(&STORE_CATEGORIES),
                _ => {}
            }
        }
    }

    Ok(())
}

/// Number of rows on the given tab (the Store's rows include category headers).
fn item_count(tab: usize) -> usize {
    match tab {
        0 => CLEANING_OPTIONS.len(),
        1 => TWEAKS_OPTIONS.len(),
        _ => pages::store::row_count(),
    }
}

/// True when `row` is a Store category header, which the cursor skips.
fn is_header(tab: usize, row: usize) -> bool {
    tab == 2 && pages::store::is_header(row)
}

/// The first selectable row on `tab`.
fn first_row(tab: usize) -> usize {
    (0..item_count(tab))
        .find(|&row| !is_header(tab, row))
        .unwrap_or(0)
}

/// The last selectable row on `tab`.
fn last_row(tab: usize) -> usize {
    (0..item_count(tab))
        .rev()
        .find(|&row| !is_header(tab, row))
        .unwrap_or(0)
}

/// Move the selection by `step` rows, skipping Store category headers.
fn step_selection(tab: usize, state: &mut TableState, step: isize) {
    let count = item_count(tab) as isize;
    let direction = step.signum();
    if count == 0 || direction == 0 {
        return;
    }
    let mut row = (state.selected().unwrap_or(0) as isize + step).clamp(0, count - 1);
    while is_header(tab, row as usize) {
        row += direction;
        if row < 0 || row >= count {
            return;
        }
    }
    state.select(Some(row as usize));
}

/// Which tab a column falls on, if any. Mirrors how `Tabs` lays titles out:
/// each title is followed by `TAB_DIVIDER`.
fn tab_index_at(tab_manager: &TabManager, column: u16, tab_area: Rect) -> Option<usize> {
    let mut start = tab_area.x;
    for (index, title) in tab_manager.titles().iter().enumerate() {
        let end = start + title.chars().count() as u16;
        if column >= start && column < end {
            return Some(index);
        }
        start = end + TAB_DIVIDER.chars().count() as u16;
    }
    None
}

/// Apply a mouse event: the wheel moves the selection, a left click either
/// switches tabs or selects the clicked row.
fn handle_mouse(
    mouse: &event::MouseEvent,
    hits: &HitAreas,
    tab_manager: &mut TabManager,
    table_state: &mut TableState,
) {
    let position = Position::new(mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            step_selection(tab_manager.get_selection(), table_state, -1);
        }
        MouseEventKind::ScrollDown => {
            step_selection(tab_manager.get_selection(), table_state, 1);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if hits.tab.contains(position) {
                // As in `run`, the nested `if let` + `if` is deliberately left
                // as two plain blocks instead of a let-chain for readability.
                #[allow(clippy::collapsible_if)]
                if let Some(index) = tab_index_at(tab_manager, mouse.column, hits.tab) {
                    if index != tab_manager.get_selection() {
                        tab_manager.select(index);
                        let first = first_row(index);
                        table_state.select(Some(first));
                    }
                }
            } else if hits.list.contains(position) {
                let row_in_view = (mouse.row - hits.list.y) as usize;
                let index = table_state.offset() + row_in_view;
                let tab = tab_manager.get_selection();
                if index < item_count(tab) && !is_header(tab, index) {
                    table_state.select(Some(index));
                }
            }
        }
        _ => {}
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
    statuses: Statuses<'_>,
    is_admin: bool,
    status: StatusLine,
) -> HitAreas {
    let Statuses {
        cleaning: cleaning_status,
        tweaks: tweaks_status,
        store: store_status,
    } = statuses;
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
    let title_height = if show_banner { banner_height } else { 1 };

    let [title_area, rule_area, tab_area, body_area, status_area] = Layout::vertical([
        Constraint::Length(title_height),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .areas(area);

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

    Paragraph::new("─".repeat(rule_area.width as usize))
        .style(theme::dim())
        .render(rule_area, frame.buffer_mut());

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
    .divider(TAB_DIVIDER)
    .select(tab_manager.get_selection())
    .render(tab_area, frame.buffer_mut());

    match tab_manager.get_selection() {
        0 => pages::cleaning::draw(frame, body_area, table_state, cleaning_status),
        1 => pages::tweaks::draw(frame, body_area, table_state, tweaks_status),
        _ => pages::store::draw(frame, body_area, table_state, store_status),
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
    let style = if is_error {
        theme::error()
    } else {
        theme::dim()
    };
    Paragraph::new(text)
        .style(style)
        .render(message_area, frame.buffer_mut());
    Paragraph::new(KEY_HINTS)
        .alignment(Alignment::Right)
        .style(theme::dim())
        .render(hints_area, frame.buffer_mut());

    // `body_area` is split by `action_table` into a one-row rule plus the rows,
    // so the list starts one line below `body_area`.
    HitAreas {
        tab: tab_area,
        list: Rect::new(
            body_area.x,
            body_area.y + 1,
            body_area.width,
            body_area.height.saturating_sub(1),
        ),
    }
}
