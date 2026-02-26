pub enum InputType {
    Hex,
    Assembly,
}

pub fn identify_type(input: &[&str]) -> InputType {
    let full_text = input.join("");
    if full_text.trim().is_empty() || full_text.contains(':') {
        return InputType::Assembly;
    };

    if hex::decode(&normalize_hex_input(&full_text)).is_ok() {
        InputType::Hex
    } else {
        InputType::Assembly
    }
}

pub fn normalize_hex_input(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '0' && matches!(chars.peek(), Some('x' | 'X')) {
            chars.next();
            continue;
        }

        if !matches!(ch, ' ' | ',' | ';' | '\n' | '\t' | '\r') {
            normalized.push(ch);
        }
    }

    normalized
}
