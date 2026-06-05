use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use inquire::ui::{Color, RenderConfig, Styled};
use inquire::{Confirm, InquireError, Password, PasswordDisplayMode, Select, Text};
use std::fmt::Display;
use std::io::stdout;

const RED: Color = Color::Rgb { r: 220, g: 38, b: 38 };

pub fn theme() {
    let cfg = RenderConfig::default()
        .with_prompt_prefix(Styled::new("›").with_fg(RED))
        .with_highlighted_option_prefix(Styled::new("▌").with_fg(RED));
    inquire::set_global_render_config(cfg);
}

pub fn clear() {
    let _ = execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0));
    println!("\x1b[1m\x1b[38;2;220;38;38mviel\x1b[0m");
    println!();
}

pub fn select<T: Display>(prompt: &str, options: Vec<T>) -> Result<Option<T>> {
    soft(Select::new(prompt, options).with_page_size(12).prompt())
}

pub fn text(prompt: &str) -> Result<Option<String>> {
    soft(Text::new(prompt).prompt())
}

pub fn password(prompt: &str) -> Result<Option<String>> {
    soft(
        Password::new(prompt)
            .with_display_mode(PasswordDisplayMode::Masked)
            .without_confirmation()
            .prompt(),
    )
}

pub fn new_password(prompt: &str) -> Result<Option<String>> {
    soft(
        Password::new(prompt)
            .with_display_mode(PasswordDisplayMode::Masked)
            .with_custom_confirmation_message("confirm password")
            .with_custom_confirmation_error_message("the passwords do not match")
            .prompt(),
    )
}

pub fn confirm(prompt: &str) -> Result<bool> {
    Ok(soft(Confirm::new(prompt).with_default(true).prompt())?.unwrap_or(false))
}

pub fn poster(bytes: &[u8]) {
    let Ok(image) = image::load_from_memory(bytes) else {
        return;
    };
    let cfg = viuer::Config {
        width: Some(28),
        height: Some(18),
        absolute_offset: false,
        ..Default::default()
    };
    let _ = viuer::print(&image, &cfg);
    println!();
}

fn soft<T>(result: Result<T, InquireError>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
