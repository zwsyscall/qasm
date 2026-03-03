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

pub struct AppState<'a> {
    pub textareas: [TextArea<'a>; 2],
    pub selected: usize,
    pub last_time: Duration,
    pub last_size: usize,
    pub last_was_asm: bool,
    pub last_was_success: bool,
    pub config: Config,
}

impl<'a> AppState<'a> {
    pub fn new(config: Config) -> Self {
        Self {
            textareas: [TextArea::default(), TextArea::default()],
            selected: 0,
            last_time: Duration::from_nanos(0),
            last_size: 0,
            last_was_asm: true,
            last_was_success: true,
            config,
        }
    }

    // Returns the index of the unfocused text area
    pub fn unselected(&self) -> usize {
        (self.selected + 1) % 2
    }

    // Swaps the active text area
    pub fn toggle_focus(&mut self) {
        self.selected = self.unselected();
    }
}

pub fn run(args: Cli, init_syntax: output::AsmSyntax) -> io::Result<()> {
    // Prelude
    // this _cleanup is kept so if we ever panic,
    // the drop handler for this is called and the terminal is left in a usable state
    let _cleanup = TerminalCleanup;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)].as_ref());
    let horizontal_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref());

    // Initialize State
    let config = Config {
        address: args.address,
        multiline: true,
        mode: args.mode,
        syntax: init_syntax,
        hex: args.format,
    };
    let mut state = AppState::new(config);

    mod_input(
        &mut state.textareas[state.selected],
        &state.config,
        state.last_was_asm,
    );
    mod_output(
        &mut state.textareas[state.unselected()],
        &state.config,
        state.last_was_asm,
        true,
        state.last_time,
        0,
    );

    let (config_tx, config_rx) = unbounded();
    let (input_tx, input_rx) = unbounded();
    let (output_tx, output_rx) = unbounded();

    let thread_config = state.config;
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
                    state.last_was_success = success;
                    state.last_time = duration;
                    state.last_size = size;
                    state.last_was_asm = asm;

                    // Update output
                    let mut new_ta = TextArea::new(lines);
                    mod_output(
                        &mut new_ta,
                        &state.config,
                        state.last_was_asm,
                        success,
                        state.last_time,
                        size,
                    );

                    // Fix position
                    let unselected_idx = state.unselected();
                    let (x, y) = state.textareas[unselected_idx].cursor();
                    new_ta.move_cursor(CursorMove::Jump(
                        x.try_into().unwrap_or(u16::MAX),
                        y.try_into().unwrap_or(u16::MAX),
                    ));

                    // re-assign
                    state.textareas[unselected_idx] = new_ta;
                }
                WorkerResult::Failure => {
                    let unselected_idx = state.unselected();
                    fail(&mut state.textareas[unselected_idx], "Decoder failure");
                }
            }
        }

        term.draw(|f| {
            let main_chunks = vertical_layout.split(f.area());
            let io_chunks = horizontal_layout.split(main_chunks[0]);

            f.render_widget(&state.textareas[0], io_chunks[0]);
            f.render_widget(&state.textareas[1], io_chunks[1]);

            let help_text = Paragraph::new(
                " Esc/^q: Quit | ^↑/↓: ±0x100 Addr | ^←/→: Output Format | ^⇧←/→: Selected Area | ^t: Multiline | ^s: Syntax | ^x: Arch | ^c/y: Copy "
            )
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(help_text, main_chunks[1]);
        })?;

        // 60 fps
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
                    state.toggle_focus();
                    state.last_was_asm = !state.last_was_asm;
                    needs_update = true;

                    mod_output(
                        &mut state.textareas[state.unselected()],
                        &state.config,
                        state.last_was_asm,
                        state.last_was_success,
                        state.last_time,
                        state.last_size,
                    );
                }

                // Address
                Input {
                    key: Key::Up,
                    ctrl: true,
                    ..
                } => {
                    state.config.address = state.config.address.saturating_add(0x100);
                    needs_update = true;
                    config_changed = true;
                }
                Input {
                    key: Key::Down,
                    ctrl: true,
                    ..
                } => {
                    state.config.address = state.config.address.saturating_sub(0x100);
                    needs_update = true;
                    config_changed = true;
                }

                // Output formatting
                Input {
                    key: Key::Right,
                    ctrl: true,
                    ..
                } => {
                    if state.last_was_asm {
                        state.config.syntax = state.config.syntax.next();
                    } else {
                        state.config.hex = state.config.hex.next();
                    }
                    config_changed = true;
                    needs_update = true;
                }
                Input {
                    key: Key::Left,
                    ctrl: true,
                    ..
                } => {
                    if state.last_was_asm {
                        state.config.syntax = state.config.syntax.next();
                    } else {
                        state.config.hex = state.config.hex.next();
                    }
                    config_changed = true;
                    needs_update = true;
                }

                // Yank / copy
                Input {
                    key: Key::Char('y') | Key::Char('c'),
                    ctrl: true,
                    ..
                } => {
                    let selected_area = &mut state.textareas[state.selected];
                    let text_to_copy = selected_area.lines().join("\n");
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if clipboard.set_text(text_to_copy).is_ok() {
                            copied(selected_area);
                        } else {
                            fail(selected_area, "Failed copying data");
                        }
                    }
                }

                // Change syntax
                Input {
                    key: Key::Char('s'),
                    ctrl: true,
                    ..
                } => {
                    state.config.syntax = state.config.syntax.next();
                    needs_update = true;
                    config_changed = true;
                }

                // Change arch
                Input {
                    key: Key::Char('x'),
                    ctrl: true,
                    ..
                } => {
                    state.config.mode = if state.config.mode == 64 { 86 } else { 64 };
                    needs_update = true;
                    config_changed = true;
                }

                // Multiline
                Input {
                    key: Key::Char('t'),
                    ctrl: true,
                    ..
                } => {
                    state.config.multiline = !state.config.multiline;
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
                    state.textareas[state.selected].insert_char(c);
                    needs_update = true;
                }

                // Only send input to the selected box
                other_input => {
                    // This is kind of a stupid hack, but to restore the colourscheme on keypresses
                    // where only the cursor is moved, we do this.
                    // First, save the cursor
                    let c_before = state.textareas[state.selected].cursor();

                    if state.textareas[state.selected].input(other_input) {
                        needs_update = true;
                    } else {
                        // Now if the input() -> bool returned false, we didn't change the content
                        // Compare the cursor position before the move to the position now
                        if c_before != state.textareas[state.selected].cursor() {
                            // if it's changed, refresh the input box
                            mod_input(
                                &mut state.textareas[state.selected],
                                &state.config,
                                state.last_was_asm,
                            );
                        }
                    }
                }
            }

            // Send the new config to the worker
            if config_changed {
                let _ = config_tx.send(WorkerEvent::ConfigChange {
                    config: state.config,
                });
            }

            // Update the input area, send the content to the worker
            if needs_update {
                mod_input(
                    &mut state.textareas[state.selected],
                    &state.config,
                    state.last_was_asm,
                );
                let source_lines: Vec<String> = state.textareas[state.selected]
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
