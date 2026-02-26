use crate::gui::Config;
use std::time::Duration;

pub enum WorkerEvent {
    ConfigChange { config: Config },
    Exit,
}

pub struct WorkerCommand {
    pub input: Vec<String>,
}

pub enum WorkerResult {
    Success {
        duration: Duration,
        success: bool,
        // Dicates whether top says assembling or disassembling
        output_asm: bool,
        // Byte sizw
        size: usize,
        lines: Vec<String>,
    },
    Failure,
}
