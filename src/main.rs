use clap::Parser;
use std::io;

use capstone::Capstone;
use capstone::arch::{BuildsCapstone, BuildsCapstoneSyntax};
use keystone_engine::{Keystone, Mode, OptionType, OptionValue};

use crate::input::assembler::assemble_text;
use crate::input::checks::InputType;
use crate::input::disassmbler::disassemble_text;
use crate::output::{AsmSyntax, HexFormat, format_bytes};

#[derive(Parser, Debug)]
#[command(name = "qasm")]
#[command(version, about = "A quick assembler/disassembler", long_about = None)]
struct Cli {
    /// Engine architecture mode (32 / 86, 64)
    #[arg(short, long, default_value_t = 64)]
    mode: u8,

    /// Syntax  (intel, att)
    #[arg(short, long, value_enum, default_value_t = AsmSyntax::Intel)]
    syntax: AsmSyntax,

    /// Output formatting to use
    #[arg(short, long, value_enum, default_value_t = HexFormat::Pretty)]
    format: HexFormat,

    /// Address to use for disassembly
    #[arg(short, long, default_value_t = 0x0)]
    address: u64,

    /// The raw input data to evaluate (e.g., 0xc3 or "mov eax, 1")
    #[arg(allow_hyphen_values = true)]
    raw_data: Vec<String>,
}

mod gui;
mod input;
mod output;

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let (ks_mode, cs_mode) = match cli.mode {
        32 | 86 => (Mode::MODE_32, capstone::arch::x86::ArchMode::Mode32),
        64 => (Mode::MODE_64, capstone::arch::x86::ArchMode::Mode64),
        _ => panic!("Unsupported mode. Use 32 or 64."),
    };

    let (cs_syntax, ks_syntax, gui_syntax) = match cli.syntax {
        AsmSyntax::Intel => (
            capstone::arch::x86::ArchSyntax::Intel,
            OptionValue::SYNTAX_INTEL,
            output::AsmSyntax::Intel,
        ),
        AsmSyntax::Att => (
            capstone::arch::x86::ArchSyntax::Att,
            OptionValue::SYNTAX_ATT,
            output::AsmSyntax::Att,
        ),
    };

    let ks = Keystone::new(keystone_engine::Arch::X86, ks_mode)
        .expect("Failed creating keystone object");

    ks.option(OptionType::SYNTAX, ks_syntax)
        .expect("Failed setting keystone syntax");

    let cs = Capstone::new()
        .x86()
        .mode(cs_mode)
        .syntax(cs_syntax)
        .build()
        .expect("Failed creating capstone object");

    // One-off
    let raw_input = cli.raw_data.join(" ");
    if !raw_input.trim().is_empty() {
        let text_vec = vec![raw_input.as_str()];

        match input::checks::identify_type(&text_vec) {
            InputType::Hex => {
                if let Some(output_lines) = disassemble_text(&cs, text_vec, 0x0, false) {
                    println!("{}", output_lines.join("\n"))
                } else {
                    println!("Failed decoding hex");
                }
            }
            InputType::Assembly => {
                if let Some(mapped_bytes) = assemble_text(&ks, &cs, text_vec, 0x0) {
                    let output_lines: Vec<String> = mapped_bytes
                        .into_iter()
                        .map(|line_bytes| {
                            if line_bytes.is_empty() {
                                String::new()
                            } else {
                                format_bytes(&line_bytes, &cli.format)
                            }
                        })
                        .collect();
                    println!("{}", output_lines.join("\n"))
                } else {
                    println!("Failed assembling code");
                }
            }
        }
    } else {
        // TUI
        gui::run(cs, ks, cli.mode, gui_syntax)?;
    }
    Ok(())
}
