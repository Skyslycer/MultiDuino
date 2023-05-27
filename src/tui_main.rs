use std::{
    collections::HashMap,
    io,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::MoveTo,
    event::{self, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use tokio::sync::mpsc::Sender;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, BorderType, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, Tabs,
    },
    Terminal,
};

use crossterm::event::{Event as CEvent, KeyCode};

use crate::structs::{self};
use crate::LOGGER;

pub async fn init(
    tui_accounts: Arc<RwLock<HashMap<String, structs::AccountData>>>,
    tui_accounts_list: Arc<RwLock<Vec<String>>>,
) {
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Couldn't create terminal abstraction!");
    enable_raw_mode().expect("TODO");
    terminal.clear().expect("TODO");

    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(tick(tx.clone()));

    let menu_titles = vec!["Dashboard", "Logs", "Quit"];
    let mut active_menu_item = structs::MenuItem::Dashboard;
    let mut account_list_state = ListState::default();
    account_list_state.select(Some(0));

    tokio::task::spawn(async move {
        loop {
            terminal
                .draw(|rect| {
                    let size = rect.size();
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints([Constraint::Length(3), Constraint::Min(2)].as_ref())
                        .split(size);

                    let menu = menu_titles
                        .iter()
                        .map(|t| {
                            let (first, rest) = t.split_at(1);
                            Spans::from(vec![
                                Span::styled(
                                    first,
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::UNDERLINED),
                                ),
                                Span::styled(rest, Style::default().fg(Color::White)),
                            ])
                        })
                        .collect();

                    let tabs = Tabs::new(menu)
                        .select(active_menu_item.into())
                        .block(Block::default().title("MultiDuino").borders(Borders::ALL))
                        .style(Style::default().fg(Color::White))
                        .highlight_style(Style::default().fg(Color::Yellow))
                        .divider(Span::raw("|"));

                    rect.render_widget(tabs, chunks[0]);
                    match active_menu_item {
                        structs::MenuItem::Logs => rect.render_widget(render_logs(), chunks[1]),
                        structs::MenuItem::Dashboard => {
                            let accounts = tui_accounts.read().expect("TODO");
                            let account_list = tui_accounts_list.read().expect("TODO");
                            let pets_chunks = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints(
                                    [Constraint::Percentage(20), Constraint::Percentage(80)]
                                        .as_ref(),
                                )
                                .split(chunks[1]);
                            let (left, right) =
                                render_dashboard(&account_list_state, &account_list, &accounts);
                            rect.render_stateful_widget(
                                left,
                                pets_chunks[0],
                                &mut account_list_state,
                            );
                            rect.render_widget(right, pets_chunks[1]);
                        }
                    }
                })
                .expect("TODO");

            match rx.recv().await.expect("TODO") {
                structs::Event::Input(event) => match event.code {
                    KeyCode::Char('q') => {
                        execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0)).expect("TODO");
                        disable_raw_mode().expect("TODO");
                        terminal.show_cursor().expect("TODO");
                        std::process::exit(1);
                    }
                    KeyCode::Left | KeyCode::Char('d') => {
                        active_menu_item = structs::MenuItem::Dashboard
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        active_menu_item = structs::MenuItem::Logs
                    }
                    KeyCode::Down => match active_menu_item {
                        structs::MenuItem::Dashboard => {
                            if let Some(selected) = account_list_state.selected() {
                                let accounts = tui_accounts.read().expect("TODO");
                                if selected >= accounts.len() - 1 {
                                    account_list_state.select(Some(0));
                                } else {
                                    account_list_state.select(Some(selected + 1));
                                }
                            }
                        }
                        structs::MenuItem::Logs => {}
                    },
                    KeyCode::Up => match active_menu_item {
                        structs::MenuItem::Dashboard => {
                            if let Some(selected) = account_list_state.selected() {
                                if selected > 0 {
                                    account_list_state.select(Some(selected - 1));
                                } else {
                                    let accounts = tui_accounts.read().expect("TODO");
                                    account_list_state.select(Some(accounts.len() - 1));
                                }
                            }
                        }
                        structs::MenuItem::Logs => {}
                    },
                    _ => {}
                },
                structs::Event::Tick => {}
            }
        }
    })
    .await
    .expect("TODO");
}

async fn tick(tx: Sender<structs::Event<KeyEvent>>) {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(5);
    loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout).expect("poll works") {
            if let CEvent::Key(key) = event::read().expect("can read events") {
                let _ = tx.send(structs::Event::Input(key)).await.is_ok();
            }
        }

        if last_tick.elapsed() >= tick_rate && (tx.send(structs::Event::Tick).await).is_ok() {
            last_tick = Instant::now();
        }
    }
}

pub fn render_logs<'a>() -> Paragraph<'a> {
    let log_items: Vec<_> = LOGGER
        .logs()
        .iter()
        .rev()
        .map(|line| Spans::from(Span::raw(line.to_string())))
        .collect();

    Paragraph::new(log_items)
        .alignment(tui::layout::Alignment::Left)
        .block(
            tui::widgets::Block::default()
                .borders(tui::widgets::Borders::ALL)
                .style(tui::style::Style::default().fg(tui::style::Color::White))
                .title("Logs")
                .border_type(tui::widgets::BorderType::Plain),
        )
}

fn render_dashboard<'a>(
    pet_list_state: &ListState,
    tui_accounts_list: &[String],
    tui_accounts: &HashMap<String, structs::AccountData>,
) -> (List<'a>, Table<'a>) {
    let pets = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Accounts")
        .border_type(BorderType::Plain);
    let items: Vec<_> = tui_accounts_list
        .iter()
        .map(|key| {
            ListItem::new(Spans::from(vec![Span::styled(
                key.clone(),
                Style::default(),
            )]))
        })
        .collect();

    let selected_name = tui_accounts_list
        .get(
            pet_list_state
                .selected()
                .expect("there is always a selected pet"),
        )
        .expect("exists")
        .clone();
    let selected_account = tui_accounts.get(&selected_name).unwrap().clone();

    let list = List::new(items).block(pets).highlight_style(
        Style::default()
            .bg(Color::Yellow)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
    );

    let pet_detail = Table::new(vec![
        Row::new(vec![
            Cell::from(Span::raw(selected_account.hashrate.to_string())),
            Cell::from(Span::raw(selected_account.miners.to_string())),
            Cell::from(Span::raw(selected_account.connected.to_string())),
            Cell::from(Span::raw(selected_account.current_balance.to_string())),
        ]),
        Row::new(vec![Cell::from(Span::raw(""))]),
        Row::new(vec![
            Cell::from(Span::styled(
                "Status",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Estimated Balance",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Staked",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Cell::from(Span::styled(
                "Warnings",
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ]),
        Row::new(vec![
            Cell::from(Span::raw(selected_account.status.to_string())),
            Cell::from(Span::raw(selected_account.estimated_balance.to_string())),
            Cell::from(Span::raw(selected_account.staked.to_string())),
            Cell::from(Span::raw(selected_account.warnings.to_string())),
        ]),
    ])
    .header(Row::new(vec![
        Cell::from(Span::styled(
            "Hashrate",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Miners",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Connected",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Cell::from(Span::styled(
            "Balance",
            Style::default().add_modifier(Modifier::BOLD),
        )),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Information")
            .border_type(BorderType::Plain),
    )
    .widths(&[
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ]);

    (list, pet_detail)
}
