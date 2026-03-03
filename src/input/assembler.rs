use capstone::Capstone;
use keystone_engine::Keystone;
use std::ffi::CString;

pub fn assemble_text(
    ks: &Keystone,
    cs: &Capstone,
    input: &mut [&str],
    address: u64,
) -> Option<(Vec<Vec<u8>>, usize)> {
    let mut produces_code = Vec::new();
    // Strip comments, empty lines etc
    for line in input.iter_mut() {
        let end = line.find(';').unwrap_or(line.len());
        *line = line[..end].trim();
        if line.is_empty() || line.ends_with(':') {
            produces_code.push(false);
        } else {
            produces_code.push(true);
        }
    }

    let full_text = CString::new(input.join("\n")).ok()?;
    let asm_result = ks.asm(full_text.as_c_str(), address).ok()?;

    let flat_bytes = asm_result.as_bytes();
    let size = flat_bytes.len();

    let decoded_instructions = cs.disasm_all(flat_bytes, address).ok()?;
    let mut decoded_iter = decoded_instructions.iter();
    let mut mapped_lines = Vec::new();

    for &has_code in &produces_code {
        if has_code {
            if let Some(insn) = decoded_iter.next() {
                mapped_lines.push(insn.bytes().to_vec());
            } else {
                // Fallback: Keystone generated fewer instructions than expected
                mapped_lines.push(Vec::new());
            }
        } else {
            // Comments, empty lines, and standalone labels get an empty vector
            mapped_lines.push(Vec::new());
        }
    }

    Some((mapped_lines, size))
}
