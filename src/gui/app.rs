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
use ratatui_textarea::{Input, Key, TextArea};

use super::helpers::*;
use crate::worker::message::{WorkerCommand, WorkerEvent, WorkerResult};
use crossbeam_channel::unbounded;
use std::io;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub address: u64,
    pub assembling: bool,
    pub multiline: bool,
    pub mode: u8,
    pub syntax: AsmSyntax,
    pub hex: HexFormat,
}

pub fn run(init_mode: u8, init_syntax: output::AsmSyntax) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut input_area = TextArea::default();
    let mut output_area = TextArea::default();
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref());
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());

    // State stuff
    let mut input_focused = true;
    let mut time_taken = Duration::from_nanos(0);
    let mut config = Config {
        address: 0,
        assembling: true,
        multiline: true,
        mode: init_mode,
        syntax: init_syntax,
        hex: HexFormat::Pretty,
    };

    mod_input(&mut input_area, &config);
    mod_output(&mut output_area, "None", true, time_taken, 0);

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
                    time_taken = duration;
                    config.assembling = asm;
                    let format = if asm {
                        config.syntax.as_str()
                    } else {
                        config.hex.as_str()
                    };

                    // PRO TIP: When re-rendering output on live changes, grab the old cursor
                    // position first so the user's scroll doesn't reset while they navigate!
                    let cursor_pos = output_area.cursor();

                    let mut new_ta = TextArea::new(lines);
                    mod_output(&mut new_ta, format, success, time_taken, size);

                    // Restore the cursor position (Requires ratatui-textarea >= 0.4.0)
                    new_ta.move_cursor(ratatui_textarea::CursorMove::Jump(
                        cursor_pos.0 as u16,
                        cursor_pos.1 as u16,
                    ));

                    output_area = new_ta;
                }
                WorkerResult::Failure => {
                    mod_output(&mut output_area, "None", false, time_taken, 0);
                }
            }
        }

        term.draw(|f| {
            let main_chunks = vertical_layout.split(f.area());
            let io_chunks = horizontal_layout.split(main_chunks[0]);

            f.render_widget(&input_area, io_chunks[0]);
            f.render_widget(&output_area, io_chunks[1]);

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

                // 2. TOGGLE FOCUS WITH CTRL + TAB
                Input {
                    key: Key::Right | Key::Left,
                    ctrl: true,
                    shift: true,
                    ..
                } => {
                    input_focused = !input_focused;
                    if input_focused {
                        activate_area(&mut input_area);
                        deactivate_area(&mut output_area);
                    } else {
                        activate_area(&mut output_area);
                        deactivate_area(&mut input_area);
                    }
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
                    let text_to_copy = output_area.lines().join("\n");
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if clipboard.set_text(text_to_copy).is_ok() {
                            copied(&mut output_area);
                        } else {
                            fail(&mut output_area, "Failed copying data");
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
                    input_area.insert_char(c);
                    needs_update = true;
                }

                // Only send input to the selected box
                other_input => {
                    if input_focused {
                        if input_area.input(other_input) {
                            needs_update = true;
                        }
                    } else {
                        // When output is focused, allow navigation but ignore text insertion
                        match other_input.key {
                            Key::Up
                            | Key::Down
                            | Key::Left
                            | Key::Right
                            | Key::PageUp
                            | Key::PageDown
                            | Key::Home
                            | Key::End => {
                                output_area.input(other_input);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Send the new config to the worker
            if config_changed {
                let _ = config_tx.send(WorkerEvent::ConfigChange { config });
            }

            // Update the input area, send the content to the worker
            if needs_update {
                mod_input(&mut input_area, &config);
                let input_lines: Vec<String> =
                    input_area.lines().iter().map(|s| s.to_string()).collect();

                let _ = input_tx.send(WorkerCommand { input: input_lines });
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
