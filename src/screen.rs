use crate::api::Api;
use anyhow::Result;
use image::imageops::FilterType;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::{HashMap, HashSet};
use std::io::{stdout, Stdout};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const RED: Color = Color::Rgb(220, 38, 38);

#[derive(Clone)]
pub struct Row {
    pub label: String,
    pub title: String,
    pub meta: String,
    pub body: String,
    pub poster: Option<String>,
}

pub enum Out {
    Pick(usize),
    Key(char, usize),
    Back,
}

pub type Search<T> = Arc<dyn Fn(&str) -> Vec<T> + Send + Sync>;

pub struct Screen<T> {
    pub heading: String,
    pub hint: String,
    pub items: Vec<T>,
    pub cursor: usize,
    pub row: fn(&T) -> Row,
    pub keys: Vec<char>,
    pub ctrl: Vec<char>,
    pub query: Option<String>,
    pub search: Option<Search<T>>,
    pub status: Option<String>,
}

impl<T> Screen<T> {
    pub fn new(heading: &str, hint: &str, items: Vec<T>, row: fn(&T) -> Row) -> Self {
        Self {
            heading: heading.into(),
            hint: hint.into(),
            items,
            cursor: 0,
            row,
            keys: Vec::new(),
            ctrl: Vec::new(),
            query: None,
            search: None,
            status: None,
        }
    }
}

struct Tui(Terminal<CrosstermBackend<Stdout>>);

impl Tui {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
        term.clear()?;
        Ok(Self(term))
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        let _ = self.0.show_cursor();
    }
}

struct Posters {
    cache: HashMap<String, Option<Vec<u8>>>,
    loading: HashSet<String>,
    tx: Sender<(String, Option<Vec<u8>>)>,
    rx: Receiver<(String, Option<Vec<u8>>)>,
}

impl Posters {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self { cache: HashMap::new(), loading: HashSet::new(), tx, rx }
    }

    fn want(&mut self, api: &Api, path: &str) {
        if self.cache.contains_key(path) || self.loading.contains(path) {
            return;
        }
        self.loading.insert(path.to_string());
        let api = api.clone();
        let tx = self.tx.clone();
        let path = path.to_string();
        thread::spawn(move || {
            let bytes = api.poster(&path);
            let _ = tx.send((path, bytes));
        });
    }

    fn pump(&mut self) {
        while let Ok((path, bytes)) = self.rx.try_recv() {
            self.loading.remove(&path);
            self.cache.insert(path, bytes);
        }
    }
}

pub fn run<T: Send + 'static>(api: &Api, screen: &mut Screen<T>) -> Result<Out> {
    let mut tui = Tui::enter()?;
    let mut state = ListState::default();
    let mut posters = Posters::new();
    let (results_tx, results_rx) = channel::<(String, Vec<T>)>();
    let mut typed_at: Option<Instant> = None;
    let mut sent: Option<String> = None;
    let mut busy = false;

    if let (Some(q), Some(search), true) = (&screen.query, &screen.search, screen.items.is_empty()) {
        dispatch(search, q, &results_tx);
        sent = Some(q.clone());
        busy = true;
    }

    loop {
        posters.pump();
        while let Ok((q, items)) = results_rx.try_recv() {
            if Some(&q) == screen.query.as_ref() {
                screen.items = items;
                screen.cursor = 0;
                busy = false;
            }
        }
        if let (Some(at), Some(q), Some(search)) = (typed_at, &screen.query, &screen.search) {
            if at.elapsed() > Duration::from_millis(250) && sent.as_ref() != Some(q) {
                dispatch(search, q, &results_tx);
                sent = Some(q.clone());
                busy = true;
                typed_at = None;
            }
        }
        if !screen.items.is_empty() {
            screen.cursor = screen.cursor.min(screen.items.len() - 1);
            for i in screen.cursor.saturating_sub(1)..(screen.cursor + 3).min(screen.items.len()) {
                if let Some(p) = (screen.row)(&screen.items[i]).poster {
                    posters.want(api, &p);
                }
            }
        }
        state.select(if screen.items.is_empty() { None } else { Some(screen.cursor) });

        tui.0.draw(|f| draw(f, screen, &mut state, &posters, busy))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }
        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let len = screen.items.len();
        match key.code {
            KeyCode::Char('c') if ctrl => return Ok(Out::Back),
            KeyCode::Char(c) if ctrl => {
                if screen.ctrl.contains(&c) && len > 0 {
                    return Ok(Out::Key(c, screen.cursor));
                }
            }
            KeyCode::Up => screen.cursor = step(screen.cursor, len, -1),
            KeyCode::Down => screen.cursor = step(screen.cursor, len, 1),
            KeyCode::PageUp => screen.cursor = step(screen.cursor, len, -10),
            KeyCode::PageDown => screen.cursor = step(screen.cursor, len, 10),
            KeyCode::Home => screen.cursor = 0,
            KeyCode::End => screen.cursor = len.saturating_sub(1),
            KeyCode::Enter if len > 0 => return Ok(Out::Pick(screen.cursor)),
            KeyCode::Esc => match &mut screen.query {
                Some(q) if !q.is_empty() => {
                    q.clear();
                    typed_at = Some(Instant::now() - Duration::from_secs(1));
                }
                _ => return Ok(Out::Back),
            },
            KeyCode::Backspace => {
                if let Some(q) = &mut screen.query {
                    q.pop();
                    typed_at = Some(Instant::now());
                }
            }
            KeyCode::Char(c) => match &mut screen.query {
                Some(q) => {
                    q.push(c);
                    typed_at = Some(Instant::now());
                }
                None => match c {
                    'k' => screen.cursor = step(screen.cursor, len, -1),
                    'j' => screen.cursor = step(screen.cursor, len, 1),
                    'q' => return Ok(Out::Back),
                    _ if screen.keys.contains(&c) && len > 0 => return Ok(Out::Key(c, screen.cursor)),
                    _ => {}
                },
            },
            _ => {}
        }
    }
}

fn dispatch<T: Send + 'static>(search: &Search<T>, query: &str, tx: &Sender<(String, Vec<T>)>) {
    let search = search.clone();
    let tx = tx.clone();
    let query = query.to_string();
    thread::spawn(move || {
        let items = search(&query);
        let _ = tx.send((query, items));
    });
}

fn step(cur: usize, len: usize, delta: i64) -> usize {
    if len == 0 {
        return 0;
    }
    (cur as i64 + delta).clamp(0, len as i64 - 1) as usize
}

fn draw<T>(f: &mut ratatui::Frame, screen: &Screen<T>, state: &mut ListState, posters: &Posters, busy: bool) {
    let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(f.area());
    let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(outer[0]);

    let mut left = cols[0];
    if let Some(q) = &screen.query {
        let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(cols[0]);
        let caret = if q.is_empty() { "search anything".dim() } else { q.as_str().into() };
        let input = Paragraph::new(Line::from(vec!["› ".fg(RED).bold(), caret, "▏".dim()]))
            .block(Block::default().borders(Borders::ALL).title(Span::styled(" viel ", Style::default().fg(RED).bold())));
        f.render_widget(input, parts[0]);
        left = parts[1];
    }

    let rows: Vec<ListItem> = screen.items.iter().map(|t| ListItem::new((screen.row)(t).label)).collect();
    let title = if busy { format!(" {} · searching… ", screen.heading) } else { format!(" {} ", screen.heading) };
    let list = List::new(rows)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().fg(RED).add_modifier(Modifier::BOLD))
        .highlight_symbol("▌ ");
    f.render_stateful_widget(list, left, state);

    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(cols[1]);
    f.render_widget(block, cols[1]);
    if let Some(item) = screen.items.get(screen.cursor) {
        detail(f, inner, &(screen.row)(item), posters);
    } else if !busy && screen.query.as_ref().is_some_and(|q| !q.is_empty()) {
        f.render_widget(Paragraph::new("nothing found".dim()), inner);
    }

    let footer = match &screen.status {
        Some(s) => Line::from(vec![Span::styled(format!(" {s}"), Style::default().fg(RED)), Span::raw("   "), screen.hint.as_str().dim()]),
        None => Line::from(format!(" {}", screen.hint).dim()),
    };
    f.render_widget(Paragraph::new(footer), outer[1]);
}

fn detail(f: &mut ratatui::Frame, area: Rect, row: &Row, posters: &Posters) {
    let poster_h = ((area.width as u32 * 3 / 4) as u16).min(area.height.saturating_sub(5));
    let panes = Layout::vertical([Constraint::Length(poster_h), Constraint::Min(0)]).split(area);
    if let Some(Some(bytes)) = row.poster.as_ref().and_then(|p| posters.cache.get(p)) {
        f.render_widget(Paragraph::new(poster_lines(bytes, area.width, poster_h)), panes[0]);
    }
    let text = vec![
        Line::from(""),
        Line::from(row.title.clone().bold()),
        Line::from(row.meta.clone().dim()),
        Line::from(""),
        Line::from(row.body.clone()),
    ];
    f.render_widget(Paragraph::new(text).wrap(Wrap { trim: true }), panes[1]);
}

pub fn countdown(label: &str, seconds: u64) -> Result<bool> {
    let mut tui = Tui::enter()?;
    let started = Instant::now();
    loop {
        let left = seconds.saturating_sub(started.elapsed().as_secs());
        tui.0.draw(|f| {
            let area = f.area();
            let y = area.height / 2;
            let lines = vec![
                Line::from("up next".dim()).centered(),
                Line::from(label.to_string().bold()).centered(),
                Line::from(""),
                Line::from(format!("playing in {left}s   ⏎ now   esc stop").dim()).centered(),
            ];
            f.render_widget(Paragraph::new(lines), Rect::new(area.x, y.saturating_sub(2), area.width, 4));
        })?;
        if left == 0 {
            return Ok(true);
        }
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Enter => return Ok(true),
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(false),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(false),
                    _ => {}
                }
            }
        }
    }
}

fn poster_lines(bytes: &[u8], max_w: u16, max_h: u16) -> Vec<Line<'static>> {
    if max_w == 0 || max_h == 0 {
        return Vec::new();
    }
    let img = match image::load_from_memory(bytes) {
        Ok(i) => i.to_rgb8(),
        Err(_) => return Vec::new(),
    };
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return Vec::new();
    }
    let target_w = max_w as u32;
    let target_h = max_h as u32 * 2;
    let scale = (target_w as f32 / iw as f32).min(target_h as f32 / ih as f32);
    let nw = ((iw as f32 * scale) as u32).max(1);
    let nh = ((ih as f32 * scale) as u32).max(2);
    let resized = image::imageops::resize(&img, nw, nh, FilterType::Lanczos3);
    let pad = (target_w.saturating_sub(nw) / 2) as usize;

    let mut lines = Vec::new();
    let mut y = 0;
    while y + 1 < nh {
        let mut spans = Vec::new();
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
        for x in 0..nw {
            let top = resized.get_pixel(x, y);
            let bottom = resized.get_pixel(x, y + 1);
            spans.push(Span::styled(
                "▀",
                Style::default()
                    .fg(Color::Rgb(top[0], top[1], top[2]))
                    .bg(Color::Rgb(bottom[0], bottom[1], bottom[2])),
            ));
        }
        lines.push(Line::from(spans));
        y += 2;
    }
    lines
}
