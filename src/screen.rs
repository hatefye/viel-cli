use crate::api::Api;
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Layout, Rect, Size};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::{Image, Resize};
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
        execute!(stdout(), Clear(ClearType::All))?;
        Ok(Self(Terminal::new(CrosstermBackend::new(stdout()))?))
    }
}

pub fn init_images() {
    picker();
}

fn picker() -> &'static Picker {
    static PICKER: std::sync::OnceLock<Picker> = std::sync::OnceLock::new();
    PICKER.get_or_init(|| {
        use std::io::IsTerminal;
        let forced = std::env::var("VIEL_IMAGES").unwrap_or_default().to_lowercase();
        let tty = std::io::stdin().is_terminal() && stdout().is_terminal();
        let mut picker = match forced.as_str() {
            "halfblocks" | "off" | "0" => Picker::halfblocks(),
            _ if !tty || (forced.is_empty() && !graphics_terminal()) => Picker::halfblocks(),
            _ => Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks()),
        };
        match forced.as_str() {
            "sixel" => picker.set_protocol_type(ProtocolType::Sixel),
            "kitty" => picker.set_protocol_type(ProtocolType::Kitty),
            "iterm2" => picker.set_protocol_type(ProtocolType::Iterm2),
            _ => {}
        }
        picker
    })
}

fn graphics_terminal() -> bool {
    let var = |k: &str| std::env::var(k).unwrap_or_default();
    if !var("WT_SESSION").is_empty() || !var("KITTY_WINDOW_ID").is_empty() || !var("KONSOLE_VERSION").is_empty() || !var("WEZTERM_EXECUTABLE").is_empty() {
        return true;
    }
    let program = var("TERM_PROGRAM").to_lowercase();
    let term = var("TERM").to_lowercase();
    ["wezterm", "iterm", "ghostty", "mintty", "rio", "warpterminal", "tabby", "contour"].iter().any(|p| program.contains(p))
        || ["kitty", "ghostty", "foot", "contour", "mlterm", "st-"].iter().any(|t| term.contains(t))
}

pub fn image_protocol() -> String {
    format!("{:?}", picker().protocol_type()).to_lowercase()
}

impl Drop for Tui {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        let _ = self.0.show_cursor();
    }
}

type Key = (String, u16, u16);
type ByteCache = HashMap<String, Option<Arc<Vec<u8>>>>;

struct Posters {
    bytes: Arc<std::sync::Mutex<ByteCache>>,
    ready: HashMap<Key, Option<(Protocol, Size)>>,
    loading: HashSet<Key>,
    tx: Sender<(Key, Option<(Protocol, Size)>)>,
    rx: Receiver<(Key, Option<(Protocol, Size)>)>,
}

impl Posters {
    fn new() -> Self {
        let (tx, rx) = channel();
        Self { bytes: Arc::default(), ready: HashMap::new(), loading: HashSet::new(), tx, rx }
    }

    fn want(&mut self, api: &Api, path: &str, size: Size) {
        let key = (path.to_string(), size.width, size.height);
        if self.ready.contains_key(&key) || self.loading.contains(&key) || size.width == 0 || size.height == 0 {
            return;
        }
        self.loading.insert(key.clone());
        let api = api.clone();
        let tx = self.tx.clone();
        let bytes = self.bytes.clone();
        let path = path.to_string();
        thread::spawn(move || {
            let cached = bytes.lock().unwrap().get(&path).cloned();
            let data = match cached {
                Some(d) => d,
                None => {
                    let fetched = api.poster(&path).map(Arc::new);
                    bytes.lock().unwrap().insert(path.clone(), fetched.clone());
                    fetched
                }
            };
            let resize = Resize::Fit(Some(image::imageops::FilterType::Lanczos3));
            let proto = data.and_then(|d| image::load_from_memory(&d).ok()).and_then(|img| {
                let cells = resize.size_for(&img, picker().font_size(), size);
                picker().new_protocol(img, size, resize).ok().map(|p| (p, cells))
            });
            let _ = tx.send((key, proto));
        });
    }

    fn pump(&mut self) {
        while let Ok((key, proto)) = self.rx.try_recv() {
            self.loading.remove(&key);
            self.ready.insert(key, proto);
        }
    }
}

fn poster_area(area: Rect) -> Rect {
    let h = ((area.width as u32 * 3 / 4) as u16).min(area.height.saturating_sub(5));
    Rect::new(area.x, area.y, area.width, h)
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
            let size = {
                let area = tui.0.size()?;
                let cols = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(Rect::new(0, 0, area.width, area.height.saturating_sub(1)));
                let inner = Block::default().borders(Borders::ALL).inner(cols[1]);
                poster_area(inner).as_size()
            };
            for i in screen.cursor.saturating_sub(1)..(screen.cursor + 3).min(screen.items.len()) {
                if let Some(p) = (screen.row)(&screen.items[i]).poster {
                    posters.want(api, &p, size);
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
    let top = poster_area(area);
    let panes = Layout::vertical([Constraint::Length(top.height), Constraint::Min(0)]).split(area);
    if let Some(Some((proto, cells))) = row.poster.as_ref().and_then(|p| posters.ready.get(&(p.clone(), top.width, top.height))) {
        let w = cells.width.min(top.width);
        let x = top.x + (top.width - w) / 2;
        f.render_widget(Image::new(proto), Rect::new(x, top.y, w, cells.height.min(top.height)));
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
