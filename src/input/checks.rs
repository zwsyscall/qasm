#[derive(Debug, PartialEq)]
pub enum InputType {
    Hex,
    Assembly,
}

pub fn identify_type(input: &[&str]) -> InputType {
    let full_text = input.join("\n");
    if full_text.trim().is_empty() {
        return InputType::Assembly;
    };

    if full_text.contains(':') && !full_text.contains("let ") {
        return InputType::Assembly;
    };

    let normalized = normalize_hex_input(&full_text);
    if !normalized.is_empty() && hex::decode(&normalized).is_ok() {
        InputType::Hex
    } else {
        InputType::Assembly
    }
}

pub fn normalize_hex_input(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    for line in input.lines() {
        let mut current_line = line.trim();
        if current_line.is_empty() {
            continue;
        }

        // These are cheap ways to identify the type but they work, so I don't really mind
        if current_line.starts_with("let ") {
            if let Some(eq_idx) = current_line.find('=') {
                if let (Some(start_offset), Some(end)) =
                    (current_line[eq_idx..].find('['), current_line.rfind(']'))
                {
                    let start = eq_idx + start_offset;
                    if start < end {
                        current_line = &current_line[start + 1..end];
                    }
                }
            }
        } else if current_line.starts_with("unsigned char ") {
            if let (Some(start), Some(end)) = (current_line.find('{'), current_line.rfind('}')) {
                if start < end {
                    current_line = &current_line[start + 1..end];
                }
            }
        }

        let mut chars = current_line.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '0' && matches!(chars.peek(), Some('x' | 'X')) {
                chars.next();
                continue;
            }

            if ch == '\\' && matches!(chars.peek(), Some('x' | 'X')) {
                chars.next();
                continue;
            }

            if !matches!(
                ch,
                ' ' | ',' | ';' | '\n' | '\t' | '\r' | '{' | '}' | '[' | ']' | '"' | '\'' | '\\'
            ) {
                normalized.push(ch);
            }
        }
    }

    normalized
}
