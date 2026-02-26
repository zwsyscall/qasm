use clap::ValueEnum;
use std::fmt;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsmSyntax {
    Intel,
    Att,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HexFormat {
    Pretty,
    RawHex,
    RustVector,
    CStyleArray,
    StringLiteral,
}

impl fmt::Display for AsmSyntax {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Intel => write!(f, "intel"),
            Self::Att => write!(f, "att"),
        }
    }
}

impl fmt::Display for HexFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pretty => write!(f, "pretty"),
            Self::RawHex => write!(f, "raw-hex"),
            Self::RustVector => write!(f, "rust-vector"),
            Self::CStyleArray => write!(f, "c-style-array"),
            Self::StringLiteral => write!(f, "string-literal"),
        }
    }
}

impl AsmSyntax {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Intel => "Intel",
            Self::Att => "AT&T",
        }
    }
}

impl Into<capstone::Syntax> for AsmSyntax {
    fn into(self) -> capstone::Syntax {
        match self {
            AsmSyntax::Intel => capstone::Syntax::Intel,
            AsmSyntax::Att => capstone::Syntax::Att,
        }
    }
}

impl HexFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pretty => "Pretty",
            Self::RawHex => "Raw hex",
            Self::StringLiteral => "String literal",
            Self::RustVector => "Rust",
            Self::CStyleArray => "C",
        }
    }
    pub fn next(&self) -> Self {
        match self {
            Self::Pretty => Self::RawHex,
            Self::RawHex => Self::RustVector,
            Self::RustVector => Self::CStyleArray,
            Self::CStyleArray => Self::StringLiteral,
            Self::StringLiteral => Self::Pretty,
        }
    }

    pub fn last(&self) -> Self {
        match self {
            Self::Pretty => Self::StringLiteral,
            Self::RawHex => Self::Pretty,
            Self::RustVector => Self::RawHex,
            Self::CStyleArray => Self::RustVector,
            Self::StringLiteral => Self::CStyleArray,
        }
    }
}

fn join_hex_bytes(bytes: &[u8], prefix: &str, sep: &str) -> String {
    bytes
        .iter()
        .map(|byte| format!("{prefix}{byte:02X}"))
        .collect::<Vec<_>>()
        .join(sep)
}

pub fn format_bytes(b: &[u8], format: &HexFormat) -> String {
    match format {
        HexFormat::Pretty => join_hex_bytes(b, "0x", ", "),
        HexFormat::RawHex => hex::encode(b),
        HexFormat::RustVector => {
            let byte_strings = join_hex_bytes(b, "0x", ", ");
            format!("let data: [u8; {}] = [{}];", b.len(), byte_strings)
        }
        HexFormat::CStyleArray => {
            let byte_strings = join_hex_bytes(b, "0x", ", ");
            format!("unsigned char data[{}] = {{ {} }};", b.len(), byte_strings)
        }
        HexFormat::StringLiteral => {
            let byte_strings = join_hex_bytes(b, "0x", ", ");
            format!("\"{}\"", byte_strings)
        }
    }
}
