use capstone::Capstone;

pub fn disassemble_text(
    cs: &Capstone,
    input: Vec<&str>,
    address: u64,
    multiline: bool,
) -> Option<Vec<String>> {
    let mut result = Vec::new();
    let mut single_line_statements = Vec::new();
    let mut current_addr = address;

    for line in input {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if multiline {
                result.push(String::new());
            }
            continue;
        }

        let clean_hex = trimmed.replace("0x", "").replace(" ", "").replace(",", "");

        if let Ok(bytes) = hex::decode(&clean_hex) {
            if let Ok(instructions) = cs.disasm_all(&bytes, current_addr) {
                for insn in instructions.iter() {
                    let mnemonic = insn.mnemonic().unwrap_or("???");
                    let op_str = insn.op_str().unwrap_or("");

                    let statement = if op_str.is_empty() {
                        mnemonic.to_string()
                    } else {
                        format!("{} {}", mnemonic, op_str)
                    };

                    if multiline {
                        result.push(statement);
                    } else {
                        single_line_statements.push(statement);
                    }
                }

                current_addr += bytes.len() as u64;
            } else {
                // Capstone failure
                return None;
            }
        } else {
            // Invalid hex
            return None;
        }
    }

    if !multiline {
        if single_line_statements.is_empty() {
            return Some(vec![String::new()]);
        } else {
            return Some(vec![single_line_statements.join("; ")]);
        }
    }

    Some(result)
}
