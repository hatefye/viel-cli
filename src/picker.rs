use crate::api::{Api, Title};
use anyhow::Result;
use image::imageops::FilterType;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::io::stdout;

const RED: Color = Color::Rgb(220, 38, 38);

pub fn pick_title(api: &Api, prompt: &str, items: Vec<Title>) -> Result<Option<Title>> {
    if items.is_empty() {
        return Ok(None);
    }
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
    let chosen = run(&mut term, api, prompt, &items);
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    let _ = term.show_cursor();
    Ok(chosen?.and_then(|i| items.into_iter().nth(i)))
}

fn run<B: ratatui::backend::Backend>(
    term: &mut Terminal<B>,
    api: &Api,
    prompt: &str,
    items: &[Title],
) -> Result<Option<usize>> {
    let mut state = ListState::default();
    state.select(Some(0));
    let mut cache: HashMap<usize, Option<Vec<u8>>> = HashMap::new();

    loop {
        let sel = state.selected().unwrap_or(0);
        cache
            .entry(sel)
            .or_insert_with(|| items[sel].poster_path.as_ref().and_then(|p| api.poster(p)));

        term.draw(|f| {
            let cols = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(f.area());

            let rows: Vec<ListItem> = items.iter().map(|t| ListItem::new(t.to_string())).collect();
            let list = List::new(rows)
                .block(Block::default().borders(Borders::ALL).title(format!(" {prompt} ")))
                .highlight_style(Style::default().fg(RED).add_modifier(Modifier::BOLD))
                .highlight_symbol("▌ ");
            f.render_stateful_widget(list, cols[0], &mut state);

            let block = Block::default().borders(Borders::ALL);
            let inner = block.inner(cols[1]);
            f.render_widget(block, cols[1]);

            let title = &items[sel];
            let poster_h = ((inner.width as u32 * 3 / 4) as u16).min(inner.height.saturating_sub(5));
            let panes = Layout::vertical([Constraint::Length(poster_h), Constraint::Min(0)]).split(inner);

            if let Some(Some(bytes)) = cache.get(&sel) {
                let lines = poster_lines(bytes, inner.width, poster_h);
                f.render_widget(Paragraph::new(lines), panes[0]);
            }

            let meta = if title.rating > 0.0 {
                format!("{} · {} · ★{:.1}", title.year, title.kind, title.rating)
            } else {
                format!("{} · {}", title.year, title.kind)
            };
            let detail = vec![
                Line::from(title.name.clone().bold()),
                Line::from(meta.dim()),
                Line::from(""),
                Line::from(title.overview.clone().unwrap_or_default()),
                Line::from(""),
                Line::from("↑↓ move   ⏎ select   esc back".dim()),
            ];
            f.render_widget(Paragraph::new(detail).wrap(Wrap { trim: true }), panes[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => step(&mut state, items.len(), -1),
                KeyCode::Down | KeyCode::Char('j') => step(&mut state, items.len(), 1),
                KeyCode::Enter => return Ok(state.selected()),
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                _ => {}
            }
        }
    }
}

fn step(state: &mut ListState, len: usize, delta: i64) {
    if len == 0 {
        return;
    }
    let cur = state.selected().unwrap_or(0) as i64;
    let next = (cur + delta).rem_euclid(len as i64) as usize;
    state.select(Some(next));
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
    let resized = image::imageops::resize(&img, nw, nh, FilterType::Triangle);
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
