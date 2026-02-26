use std::time::Duration;

use crate::gui::app::Config;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui_textarea::TextArea;

pub fn mod_output(
    textarea: &mut TextArea<'_>,
    format: &str,
    success: bool,
    taken: Duration,
    size: usize,
) {
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    let style = if success {
        Style::default().green()
    } else {
        Style::default().red()
    };
    let title = { format!("| Format {} | {} bytes | Took {:#?} |", format, size, taken,) };

    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(style)
            .title(title),
    );
}

pub fn copied(textarea: &mut TextArea<'_>) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().cyan())
            .title(" Copied output! "),
    );
}

pub fn fail(textarea: &mut TextArea<'_>, msg: &str) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().cyan())
            .title(format!(" {} ", msg)),
    );
}

pub fn mod_input(textarea: &mut TextArea<'_>, config: &Config) {
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
    let multiline = if config.multiline { "ON" } else { "OFF" };
    let action = if config.assembling {
        "Assembling"
    } else {
        "Disassembling"
    };
    let title = {
        format!(
            "| {} | multiline {} | x{} | {} | offset {:#X} |",
            action,
            multiline,
            config.mode,
            config.syntax.as_str(),
            config.address
        )
    };
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default())
            .title(title),
    );
}

pub fn activate_area(textarea: &mut TextArea<'_>) {
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
}

pub fn deactivate_area(textarea: &mut TextArea<'_>) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
}
