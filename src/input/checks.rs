pub enum InputType {
    Hex,
    Assembly,
}

pub fn can_analyze(input_lines: &[&str]) -> bool {
    for &line in input_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lower = trimmed.to_lowercase();

        let mut bracket_depth = 0;
        let mut paren_depth = 0;

        for ch in lower.chars() {
            match ch {
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                _ => {}
            }

            if bracket_depth < 0 || paren_depth < 0 {
                return false;
            }
        }

        if bracket_depth != 0 || paren_depth != 0 {
            return false;
        }
        if lower.starts_with('+')
            || lower.starts_with('-')
            || lower.starts_with('*')
            || lower.starts_with('/')
            || lower.starts_with(',')
            || lower.ends_with(',')
            || lower.ends_with('+')
            || lower.ends_with('-')
            || lower.ends_with('*')
        {
            return false;
        }

        let bytes = lower.as_bytes();
        for i in 0..bytes.len() {
            if bytes[i] == b'0' && i + 1 < bytes.len() && bytes[i + 1] == b'x' {
                if i + 2 >= bytes.len() {
                    return false;
                }

                let next_ch = bytes[i + 2];
                if !next_ch.is_ascii_hexdigit() {
                    return false;
                }
            }
        }
    }

    true
}

pub fn identify_type(input: &[&str]) -> InputType {
    let full_text = input.join("");
    if full_text.trim().is_empty() || full_text.contains(":") {
        return InputType::Assembly;
    }

    let clean_text = full_text
        .replace("0x", "")
        .replace(" ", "")
        .replace(",", "")
        .replace(";", "")
        .replace("\n", "");

    if hex::decode(&clean_text).is_ok() {
        InputType::Hex
    } else {
        InputType::Assembly
    }
}
