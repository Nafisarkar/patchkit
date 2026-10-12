use color_eyre::eyre::{Ok, Result};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Padding, Paragraph, TableState, Tabs, Widget},
};

mod data;
mod pages;
mod tab;
mod tab_manager;
use tab_manager::TabManager;

fn main() -> Result<()> {
    let mut manager = TabManager::new();
    manager.add_tab("Cleaning");
    manager.add_tab("Tweaks");

    let mut table_state = TableState::default();
    table_state.select(Some(0));

    manager
        .get_tabs()
        .iter()
        .for_each(|f| println!("{} - {}", f.title, f.tabnumber));

    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut manager, &mut table_state);
    ratatui::restore();
    result
}

fn run(
    terminal: &mut DefaultTerminal,
    tabmanager: &mut TabManager,
    table_state: &mut TableState,
) -> Result<()> {
    loop {
        // draw
        terminal.draw(|f| draw(f, tabmanager, table_state))?;

        // event handler
        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                continue;
            }
            match key.code {
                event::KeyCode::Esc | KeyCode::Char('q') => {
                    break;
                }

                // table nav
                KeyCode::Up | KeyCode::Char('k') => {
                    let selected = table_state.selected().unwrap_or(0);

                    if selected > 0 {
                        table_state.select(Some(selected - 1));
                    }
                }

                KeyCode::Down | KeyCode::Char('j') => {
                    let selected = table_state.selected().unwrap_or(0);

                    if selected + 1 < 3 {
                        table_state.select(Some(selected + 1));
                    }
                }

                KeyCode::Enter => {
                    if tabmanager.get_selection() == 0 {
                        if let Some(index) = table_state.selected() {
                            match index {
                                0 => {
                                    println!("Clean Temp");
                                }
                                1 => {
                                    println!("Clean Cache");
                                }
                                2 => {
                                    println!("Clean Recycle Bin");
                                }
                                _ => {}
                            }
                        }
                    } else if tabmanager.get_selection() == 1 {
                    }
                }

                // tab nav
                event::KeyCode::Left | KeyCode::Char('h') => {
                    tabmanager.prev_tab();
                }
                event::KeyCode::Right | KeyCode::Char('l') => {
                    tabmanager.next_tab();
                }

                _ => {}
            }
        }
    }
    Ok(())
}

fn draw(frame: &mut Frame, tabmanager: &TabManager, table_state: &mut TableState) {
    let [headarea, tabsarea, bodyarea] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(frame.area());

    Paragraph::new("PatchKit")
        .block(Block::bordered().padding(Padding::horizontal(1)))
        .render(headarea, frame.buffer_mut());

    Tabs::new(
        tabmanager
            .get_tabs()
            .iter()
            .map(|tab| tab.title.as_str())
            .collect::<Vec<_>>(),
    )
    .style(Style::default().fg(Color::Gray))
    .highlight_style(
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    )
    .select(tabmanager.get_selection())
    .render(tabsarea, frame.buffer_mut());

    match tabmanager.get_selection() {
        0 => pages::cleaning::draw(frame, bodyarea, table_state),
        1 => pages::tweaks::draw(frame, bodyarea, table_state),
        _ => {}
    }
}
