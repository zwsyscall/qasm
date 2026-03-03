use std::time::Duration;

use crate::gui::app::Config;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui_textarea::TextArea;

pub fn mod_output(
    textarea: &mut TextArea<'_>,
    config: &Config,
    asm: bool,
    success: bool,
    taken: Duration,
    size: usize,
) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    let style = if success {
        Style::default().green()
    } else {
        Style::default().red()
    };
    let output_type = if asm {
        format!("{} | x{}", config.syntax.as_str(), config.mode)
    } else {
        let multiline = if config.multiline {
            "multiline: ON"
        } else {
            "multiline: OFF"
        };
        format!("{} | {}", config.hex.as_str(), multiline)
    };

    let title = format!("| {} | {} bytes | Took {:#?} |", output_type, size, taken);

    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(style)
            .title(title),
    );
}

pub fn mod_input(textarea: &mut TextArea<'_>, config: &Config, asm: bool) {
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));

    let input_type = if asm {
        "Disassembling".to_string()
    } else {
        format!("{} | x{}", config.syntax.as_str(), config.mode)
    };

    let title = format!("| {} | offset {:#X} |", input_type, config.address);

    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default())
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
            .title(" Copied content! "),
    );
}

pub fn fail(textarea: &mut TextArea<'_>, msg: &str) {
    textarea.set_cursor_line_style(Style::default());
    textarea.set_cursor_style(Style::default());
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().light_red())
            .title(format!(" {} ", msg)),
    );
}
