use crate::Cli;
use crate::output::{self, AsmSyntax, HexFormat};
use ratatui::Terminal;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::CrosstermBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui_textarea::{CursorMove, Input, Key, TextArea};

use super::helpers::*;
use crate::worker::message::{WorkerCommand, WorkerEvent, WorkerResult};
use crossbeam_channel::unbounded;
use std::thread;
use std::time::Duration;
use std::{io, u16};

// Useful for panics
struct TerminalCleanup;

impl Drop for TerminalCleanup {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub address: u64,
    pub multiline: bool,
    pub mode: u8,
    pub syntax: AsmSyntax,
    pub hex: HexFormat,
}

pub fn run(args: Cli, init_syntax: output::AsmSyntax) -> io::Result<()> {
    let _cleanup = TerminalCleanup;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut textarea = [TextArea::default(), TextArea::default()];
    let mut selected = 0;
    let mut unselected = (selected + 1) % 2;

    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref());
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());

    // State stuff
    let mut last_time = Duration::from_nanos(0);
    let mut last_size = 0;
    let mut last_was_asm = true;
    let mut last_was_success = true;
    let mut config = Config {
        address: args.address,
        multiline: true,
        mode: args.mode,
        syntax: init_syntax,
        hex: args.format,
    };

    mod_input(&mut textarea[selected], &config, last_was_asm);
    mod_output(&mut textarea[unselected], "None", true, last_time, 0);

    let (config_tx, config_rx) = unbounded();
    let (input_tx, input_rx) = unbounded();
    let (output_tx, output_rx) = unbounded();

    let thread_config = config;
    thread::spawn(move || {
        crate::worker::run(&thread_config, config_rx, input_rx, output_tx);
    });

    loop {
        while let Ok(res) = output_rx.try_recv() {
            match res {
                WorkerResult::Success {
                    lines,
                    success,
                    output_asm: asm,
                    size,
                    duration,
                } => {
                    last_was_success = success;
                    last_time = duration;
                    last_size = size;
                    last_was_asm = asm;

                    let format = if asm {
                        config.syntax.as_str()
                    } else {
                        config.hex.as_str()
                    };

                    // Update output
                    let mut new_ta = TextArea::new(lines);
                    mod_output(&mut new_ta, format, success, last_time, size);

                    // Fix position
                    let (x, y) = textarea[unselected].cursor();
                    new_ta.move_cursor(CursorMove::Jump(
                        x.try_into().unwrap_or(u16::MAX),
                        y.try_into().unwrap_or(u16::MAX),
                    ));

                    // re-assign
                    textarea[unselected] = new_ta;
                }
                WorkerResult::Failure => {
                    fail(&mut textarea[unselected], "Failed translating data");
                }
            }
        }

        term.draw(|f| {
            let main_chunks = vertical_layout.split(f.area());
            let io_chunks = horizontal_layout.split(main_chunks[0]);

            f.render_widget(&textarea[0], io_chunks[0]);
            f.render_widget(&textarea[1], io_chunks[1]);

            let help_text = Paragraph::new(
                " Esc/^q: Quit | ^Up/Down: ±0x100 Addr | ^←/→ : Output format | ^←/→: Selected area | ^t: multiline | ^s: Syntax | ^x: Arch | ^c/y: Copy "
            )
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(help_text, main_chunks[1]);
        })?;

        if ratatui::crossterm::event::poll(Duration::from_millis(16))? {
            let mut needs_update = false;
            let mut config_changed = false;

            match ratatui::crossterm::event::read()?.into() {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('q'),
                    ctrl: true,
                    ..
                } => {
                    let _ = config_tx.send(WorkerEvent::Exit);
                    break;
                }

                Input {
                    key: Key::Right | Key::Left,
                    ctrl: true,
                    shift: true,
                    ..
                } => {
                    let format = if !last_was_asm {
                        config.syntax.as_str()
                    } else {
                        config.hex.as_str()
                    };

                    selected = (selected + 1) % 2;
                    unselected = (selected + 1) % 2;

                    mod_input(&mut textarea[selected], &config, last_was_asm);
                    mod_output(
                        &mut textarea[unselected],
                        &format,
                        last_was_success,
                        last_time,
                        last_size,
                    );
                }

                // Address
                Input {
                    key: Key::Up,
                    ctrl: true,
                    ..
                } => {
                    config.address = config.address.saturating_add(0x100);
                    needs_update = true;
                    config_changed = true;
                }
                Input {
                    key: Key::Down,
                    ctrl: true,
                    ..
                } => {
                    config.address = config.address.saturating_sub(0x100);
                    needs_update = true;
                    config_changed = true;
                }

                // Output formatting
                Input {
                    key: Key::Right,
                    ctrl: true,
                    ..
                } => {
                    config.hex = config.hex.next();
                    config_changed = true;
                    needs_update = true;
                }
                Input {
                    key: Key::Left,
                    ctrl: true,
                    ..
                } => {
                    config.hex = config.hex.last();
                    config_changed = true;
                    needs_update = true;
                }

                // Yank / copy
                Input {
                    key: Key::Char('y') | Key::Char('c'),
                    ctrl: true,
                    ..
                } => {
                    let mut selected_area = &mut textarea[selected];
                    let text_to_copy = selected_area.lines().join("\n");
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if clipboard.set_text(text_to_copy).is_ok() {
                            copied(&mut selected_area);
                        } else {
                            fail(&mut selected_area, "Failed copying data");
                        }
                    }
                }

                // Change syntax
                Input {
                    key: Key::Char('s'),
                    ctrl: true,
                    ..
                } => {
                    config.syntax = match config.syntax {
                        output::AsmSyntax::Intel => output::AsmSyntax::Att,
                        output::AsmSyntax::Att => output::AsmSyntax::Intel,
                    };
                    needs_update = true;
                    config_changed = true;
                }

                // Change arch
                Input {
                    key: Key::Char('x'),
                    ctrl: true,
                    ..
                } => {
                    config.mode = if config.mode == 64 { 86 } else { 64 };
                    needs_update = true;
                    config_changed = true;
                }

                // Multiline
                Input {
                    key: Key::Char('t'),
                    ctrl: true,
                    ..
                } => {
                    config.multiline = !config.multiline;
                    needs_update = true;
                    config_changed = true;
                }

                // Weird keyboard language kink fixes
                Input {
                    key: Key::Char(c),
                    ctrl: true,
                    alt: true,
                    ..
                } => {
                    textarea[selected].insert_char(c);
                    needs_update = true;
                }

                // Only send input to the selected box
                other_input => {
                    if textarea[selected].input(other_input) {
                        needs_update = true;
                    }
                }
            }

            // Send the new config to the worker
            if config_changed {
                let _ = config_tx.send(WorkerEvent::ConfigChange { config });
            }

            // Update the input area, send the content to the worker
            if needs_update {
                mod_output(
                    &mut textarea[unselected],
                    if last_was_asm {
                        config.syntax.as_str()
                    } else {
                        config.hex.as_str()
                    },
                    last_was_success,
                    last_time,
                    last_size,
                );
                mod_input(&mut textarea[selected], &config, last_was_asm);
                let source_lines: Vec<String> = textarea[selected]
                    .lines()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                let _ = input_tx.send(WorkerCommand {
                    input: source_lines,
                });
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;
    Ok(())
}
