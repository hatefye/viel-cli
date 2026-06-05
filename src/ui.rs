use anyhow::Result;
use inquire::ui::{Color, RenderConfig, Styled};
use inquire::{InquireError, Select, Text};
use std::fmt::Display;

const RED: Color = Color::Rgb { r: 220, g: 38, b: 38 };

pub fn theme() {
    let cfg = RenderConfig::default()
        .with_prompt_prefix(Styled::new("›").with_fg(RED))
        .with_highlighted_option_prefix(Styled::new("▌").with_fg(RED));
    inquire::set_global_render_config(cfg);
}

pub fn select<T: Display>(prompt: &str, options: Vec<T>) -> Result<Option<T>> {
    soft(Select::new(prompt, options).with_page_size(12).prompt())
}

pub fn text(prompt: &str) -> Result<Option<String>> {
    soft(Text::new(prompt).prompt())
}

fn soft<T>(result: Result<T, InquireError>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
