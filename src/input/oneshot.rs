use std::fs;

use crate::output::{AsmSyntax, format_bytes};
use crate::{Cli, input};
use capstone::Capstone;
use capstone::arch::{BuildsCapstone, BuildsCapstoneSyntax};
use keystone_engine::{Keystone, Mode, OptionType, OptionValue};

use crate::input::InputType;
use crate::input::assembler::assemble_text;
use crate::input::disassmbler::disassemble_text;

pub fn analyze(cli: Cli, mut text_vec: String) -> String {
    // Setup
    let (ks_mode, cs_mode) = match cli.mode {
        32 | 86 => (Mode::MODE_32, capstone::arch::x86::ArchMode::Mode32),
        64 => (Mode::MODE_64, capstone::arch::x86::ArchMode::Mode64),
        _ => panic!("Unsupported mode. Use 32 or 64."),
    };
    let (cs_syntax, ks_syntax) = match cli.syntax {
        AsmSyntax::Intel => (
            capstone::arch::x86::ArchSyntax::Intel,
            OptionValue::SYNTAX_INTEL,
        ),
        AsmSyntax::Att => (
            capstone::arch::x86::ArchSyntax::Att,
            OptionValue::SYNTAX_ATT,
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

    // 'biz
    if let Some(path) = cli.input {
        let data = fs::read(path).expect("Unable to read file");
        text_vec = hex::encode(data);
    }

    match input::identify_type(&[text_vec.as_str()]) {
        InputType::Hex => {
            if let Some((output_lines, _size)) =
                disassemble_text(&cs, &[text_vec.as_str()], 0x0, true)
            {
                return output_lines.join("\n");
            } else {
                return "Failed decoding hex".to_string();
            }
        }
        InputType::Assembly => {
            if let Some((mapped_bytes, _size)) = assemble_text(&ks, &cs, &[text_vec.as_str()], 0x0)
            {
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
                return output_lines.join("\n");
            } else {
                return "Failed assembling code".to_string();
            }
        }
    }
}
