use crate::input;
use crate::input::assembler::assemble_text;
use crate::input::checks::InputType;
use crate::input::disassmbler::disassemble_text;
use crate::output::{self, AsmSyntax, HexFormat, format_bytes};
use capstone::Capstone;
use capstone::arch::x86::ArchMode;
use keystone_engine::{Keystone, Mode, OptionValue};
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
use std::io;

pub struct Config {
    pub address: u64,
    pub assembling: bool,
    pub multiline: bool,
    // arch
    pub mode: u8,
    pub asm: AsmSyntax,
    pub hex: HexFormat,
}

pub fn run(
    mut cs: Capstone,
    mut ks: Keystone,
    init_mode: u8,
    init_syntax: output::AsmSyntax,
) -> io::Result<()> {
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

    let mut config = Config {
        address: 0,
        assembling: true,
        multiline: true,
        mode: init_mode,
        asm: init_syntax,
        hex: HexFormat::Pretty,
    };

    mod_input(&mut input_area, &config);
    mod_output(&mut output_area, &config, true);

    loop {
        let mut needs_update = false;
        term.draw(|f| {
            let main_chunks = vertical_layout.split(f.area());
            let io_chunks = horizontal_layout.split(main_chunks[0]);

            f.render_widget(&input_area, io_chunks[0]);
            f.render_widget(&output_area, io_chunks[1]);

            // Footer
            let help_text = Paragraph::new(
                " Esc/^q: Quit | ^Up/Down: ±0x100 Addr | ^Left/Right: Output format | ^t: multiline | ^s: Syntax | ^x: Arch | ^c/y: Copy "
            )
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(help_text, main_chunks[1]);
        })?;
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => break,

            // Address
            Input {
                key: Key::Up,
                ctrl: true,
                ..
            } => {
                config.address = config.address.saturating_add(0x100);
                needs_update = true;
            }
            Input {
                key: Key::Down,
                ctrl: true,
                ..
            } => {
                config.address = config.address.saturating_sub(0x100);
                needs_update = true;
            }

            // Output format
            Input {
                key: Key::Right,
                ctrl: true,
                ..
            } => {
                config.hex = config.hex.next();
                needs_update = true;
            }
            Input {
                key: Key::Left,
                ctrl: true,
                ..
            } => {
                config.hex = config.hex.last();
                needs_update = true;
            }

            // Copy
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
            } => match config.asm {
                output::AsmSyntax::Intel => {
                    cs.set_syntax(capstone::Syntax::Att)
                        .expect("Failure capstone changing syntax");
                    ks.option(keystone_engine::OptionType::SYNTAX, OptionValue::SYNTAX_ATT)
                        .expect("Failure keystone changing syntax");
                    config.asm = output::AsmSyntax::Att;
                    needs_update = true;
                }
                output::AsmSyntax::Att => {
                    cs.set_syntax(capstone::Syntax::Intel)
                        .expect("Failure capstone changing syntax");
                    ks.option(
                        keystone_engine::OptionType::SYNTAX,
                        OptionValue::SYNTAX_INTEL,
                    )
                    .expect("Failure keystone changing syntax");
                    config.asm = output::AsmSyntax::Intel;
                    needs_update = true;
                }
            },

            // Change arch
            Input {
                key: Key::Char('x'),
                ctrl: true,
                ..
            } => {
                if config.mode == 64 {
                    config.mode = 86;
                    cs.set_mode(ArchMode::Mode32.into())
                        .expect("Failure changing capstone mode");
                    ks = Keystone::new(keystone_engine::Arch::X86, Mode::MODE_32)
                        .expect("Failure changing keystone mode");
                } else if config.mode == 86 || config.mode == 32 {
                    config.mode = 64;
                    cs.set_mode(ArchMode::Mode64.into())
                        .expect("Failure changing capstone mode");
                    ks = Keystone::new(keystone_engine::Arch::X86, Mode::MODE_64)
                        .expect("Failure changing keystone mode");
                }

                let syntax_opt = match config.asm {
                    output::AsmSyntax::Intel => OptionValue::SYNTAX_INTEL,
                    output::AsmSyntax::Att => OptionValue::SYNTAX_ATT,
                };
                ks.option(keystone_engine::OptionType::SYNTAX, syntax_opt)
                    .expect("Failure re-applying keystone syntax after arch change");

                needs_update = true;
            }
            // Multiline
            Input {
                key: Key::Char('t'),
                ctrl: true,
                ..
            } => {
                config.multiline = !config.multiline;
                needs_update = true
            }

            Input {
                key: Key::Char(c),
                ctrl: true,
                alt: true,
                ..
            } => {
                input_area.insert_char(c);
                needs_update = true;
            }

            other_input => {
                if input_area.input(other_input) {
                    needs_update = true;
                }
            }
        }
        if needs_update {
            mod_input(&mut input_area, &config);
            let input_lines: Vec<&str> = input_area.lines().iter().map(|s| s.as_str()).collect();

            match input::checks::identify_type(&input_lines) {
                InputType::Hex => {
                    if let Some(output_lines) =
                        disassemble_text(&cs, input_lines, config.address, config.multiline)
                    {
                        config.assembling = false;
                        let mut new_ta = TextArea::new(output_lines);
                        mod_output(&mut new_ta, &config, true);
                        output_area = new_ta;
                    } else {
                        mod_output(&mut output_area, &config, false);
                    }
                }
                InputType::Assembly => {
                    if let Some(mapped_bytes) = assemble_text(&ks, &cs, input_lines, config.address)
                    {
                        config.assembling = true;
                        let output_lines: Vec<String> = match config.multiline {
                            true => mapped_bytes
                                .into_iter()
                                .map(|line_bytes| {
                                    if line_bytes.is_empty() {
                                        String::new()
                                    } else {
                                        format_bytes(&line_bytes, &config.hex)
                                    }
                                })
                                .collect(),
                            false => vec![format_bytes(&mapped_bytes.concat(), &config.hex)],
                        };

                        let mut new_ta = TextArea::new(output_lines);
                        mod_output(&mut new_ta, &config, true);
                        output_area = new_ta;
                    } else {
                        mod_output(&mut output_area, &config, false);
                    }
                }
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
