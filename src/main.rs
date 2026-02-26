use crate::{
    input::oneshot,
    output::{AsmSyntax, HexFormat},
};
use clap::Parser;
use std::{io, path::PathBuf};

#[derive(Parser, Debug)]
#[command(name = "qasm")]
#[command(version, about = "A quick assembler/disassembler", long_about = None)]
pub struct Cli {
    /// Engine architecture mode (32 / 86, 64)
    #[arg(short, long, default_value_t = 64)]
    pub mode: u8,

    /// Syntax  (intel, att)
    #[arg(short, long, value_enum, default_value_t = AsmSyntax::Intel)]
    pub syntax: AsmSyntax,

    /// Output formatting to use
    #[arg(short, long, value_enum, default_value_t = HexFormat::Pretty)]
    pub format: HexFormat,

    /// Address to use for disassembly
    #[arg(short, long, default_value_t = 0x0)]
    pub address: u64,

    /// Parse file as input
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// The raw input data to evaluate (e.g., 0xc3 or "mov eax, 1")
    #[arg(allow_hyphen_values = true)]
    pub data: Vec<String>,
}

mod gui;
mod input;
mod output;
mod worker;

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let gui_syntax = match cli.syntax {
        AsmSyntax::Intel => output::AsmSyntax::Intel,
        AsmSyntax::Att => output::AsmSyntax::Att,
    };

    // One-off
    let raw_input = cli.data.join(" ");
    if !raw_input.trim().is_empty() || cli.input.is_some() {
        let output_text = oneshot::analyze(cli, raw_input);
        println!("{}", output_text);
    } else {
        // TUI
        gui::run(cli.mode, gui_syntax)?;
    }
    Ok(())
}
