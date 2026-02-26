pub mod assembler;
pub mod disassmbler;
pub mod oneshot;

pub enum InputType {
    Hex,
    Assembly,
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
