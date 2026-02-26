use std::time::Instant;

use crate::{
    gui::Config,
    input::{self, InputType, assembler::assemble_text, disassmbler::disassemble_text},
    output::{AsmSyntax, format_bytes},
};

use super::message::{WorkerCommand, WorkerEvent, WorkerResult};
use capstone::Capstone;
use capstone::arch::BuildsCapstone;
use crossbeam_channel::{Receiver, Sender, select};
use keystone_engine::{Keystone, OptionValue};

fn reconfigure_engines(cs: &mut Capstone, ks: &mut Keystone, config: &Config) {
    let (ks_mode, cs_mode) = match config.mode {
        32 | 86 => (
            keystone_engine::Mode::MODE_32,
            capstone::arch::x86::ArchMode::Mode32,
        ),
        64 => (
            keystone_engine::Mode::MODE_64,
            capstone::arch::x86::ArchMode::Mode64,
        ),
        _ => panic!("Unsupported mode. Use 32 or 64."),
    };

    let (cs_syntax, ks_syntax) = match config.syntax {
        AsmSyntax::Intel => (
            capstone::arch::x86::ArchSyntax::Intel,
            OptionValue::SYNTAX_INTEL,
        ),
        AsmSyntax::Att => (
            capstone::arch::x86::ArchSyntax::Att,
            OptionValue::SYNTAX_ATT,
        ),
    };

    // Fuck it we ball
    cs.set_syntax(cs_syntax.into())
        .expect("Failed setting capstone syntax");
    cs.set_mode(cs_mode.into())
        .expect("Failed setting capstone mode");

    // Fuck it we ball^2
    *ks = Keystone::new(keystone_engine::Arch::X86, ks_mode)
        .expect("Failed creating keystone object");
    ks.option(keystone_engine::OptionType::SYNTAX, ks_syntax)
        .expect("Failed setting keystone syntax");
}

// Worker thread
pub fn run(
    initial_conf: &Config,
    config_rx: Receiver<WorkerEvent>,
    input_rx: Receiver<WorkerCommand>,
    output_tx: Sender<WorkerResult>,
) {
    let mut cfg = *initial_conf;
    let cs_mode = match cfg.mode {
        32 | 86 => capstone::arch::x86::ArchMode::Mode32,
        64 => capstone::arch::x86::ArchMode::Mode64,
        _ => panic!("Unsupported mode"),
    };

    let mut cs = Capstone::new()
        .x86()
        .mode(cs_mode)
        .build()
        .expect("Failed to build initial Capstone");

    let mut ks = Keystone::new(
        keystone_engine::Arch::X86,
        if cfg.mode == 64 {
            keystone_engine::Mode::MODE_64
        } else {
            keystone_engine::Mode::MODE_32
        },
    )
    .expect("Failed to build initial Keystone");

    // Sync capstone & KS before we start
    reconfigure_engines(&mut cs, &mut ks, &cfg);

    loop {
        let mut new_config = None;
        let mut latest_cmd = None;

        // Wait for either event
        select! {
            recv(config_rx) -> msg => match msg {
                Ok(WorkerEvent::ConfigChange { config }) => new_config = Some(config),
                Ok(WorkerEvent::Exit) => return,
                Err(_) => break,
            },
            recv(input_rx) -> msg => match msg {
                Ok(cmd) => latest_cmd = Some(cmd),
                Err(_) => break,
            },
        }

        // This only gets the last config state
        while let Ok(eve) = config_rx.try_recv() {
            match eve {
                WorkerEvent::ConfigChange { config } => {
                    new_config = Some(config);
                }
                WorkerEvent::Exit => return,
            }
        }

        // And this updates to it
        if let Some(config) = new_config {
            if config != cfg {
                reconfigure_engines(&mut cs, &mut ks, &config);
                cfg = config;
            }
        }

        // Same here, this only gets the last message
        while let Ok(cmd) = input_rx.try_recv() {
            latest_cmd = Some(cmd);
        }

        // And assembles it
        if let Some(cmd) = latest_cmd {
            let now = Instant::now();
            let msg: Vec<&str> = cmd.input.iter().map(|s| s.as_str()).collect();

            let reply = match input::identify_type(&msg) {
                InputType::Hex => {
                    if let Some((output_lines, size)) =
                        disassemble_text(&cs, &msg, cfg.address, cfg.multiline)
                    {
                        let success = !output_lines.iter().all(|line| line.is_empty());
                        WorkerResult::Success {
                            duration: now.elapsed(),
                            success: success,
                            output_asm: true,
                            size: size,
                            lines: output_lines,
                        }
                    } else {
                        WorkerResult::Failure
                    }
                }
                InputType::Assembly => {
                    if let Some((mapped_bytes, size)) = assemble_text(&ks, &cs, &msg, cfg.address) {
                        let output_lines: Vec<String> = if cfg.multiline {
                            mapped_bytes
                                .into_iter()
                                .map(|line_bytes| {
                                    if line_bytes.is_empty() {
                                        String::new()
                                    } else {
                                        format_bytes(&line_bytes, &cfg.hex)
                                    }
                                })
                                .collect()
                        } else {
                            vec![format_bytes(&mapped_bytes.concat(), &cfg.hex)]
                        };

                        let success = !output_lines.iter().all(|line| line.is_empty());
                        WorkerResult::Success {
                            duration: now.elapsed(),
                            success: success,
                            output_asm: false,
                            size: size,
                            lines: output_lines,
                        }
                    } else {
                        WorkerResult::Failure
                    }
                }
            };
            // Listener disconnected
            if let Err(_) = output_tx.send(reply) {
                return;
            }
        }
    }
}
