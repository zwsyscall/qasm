use capstone::Capstone;
use keystone_engine::Keystone;
use std::ffi::CString;

pub fn assemble_text(
    ks: &Keystone,
    cs: &Capstone,
    input: &[&str],
    address: u64,
) -> Option<(Vec<Vec<u8>>, usize)> {
    let mut expected_counts = Vec::new();
    for &line in input {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.ends_with(':') {
            expected_counts.push(0);
        } else {
            let count = trimmed.split(';').filter(|s| !s.trim().is_empty()).count();
            expected_counts.push(count);
        }
    }
    let full_text = CString::new(input.join("\n")).ok()?;
    let asm_result = ks.asm(full_text.as_c_str(), address).ok()?;

    let flat_bytes = asm_result.as_bytes();
    let size = flat_bytes.len();

    let decoded_instructions = cs.disasm_all(flat_bytes, address).ok()?;
    let mut decoded_iter = decoded_instructions.iter();
    let mut mapped_lines = Vec::new();
    for expected_count in expected_counts {
        let mut line_bytes = Vec::new();
        for _ in 0..expected_count {
            if let Some(insn) = decoded_iter.next() {
                line_bytes.extend_from_slice(insn.bytes());
            }
        }

        mapped_lines.push(line_bytes);
    }

    Some((mapped_lines, size))
}
