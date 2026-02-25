use clap::ValueEnum;
use std::fmt;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
pub enum AsmSyntax {
    Intel,
    Att,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq)]
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

pub fn format_bytes(b: &[u8], format: &HexFormat) -> String {
    match format {
        HexFormat::Pretty => {
            let byte_strings = b
                .iter()
                .map(|byte| format!("0x{:02X}", byte))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}", byte_strings)
        }

        HexFormat::RawHex => hex::encode(b),

        HexFormat::RustVector => {
            let byte_strings = b
                .iter()
                .map(|byte| format!("0x{:02X}", byte))
                .collect::<Vec<_>>()
                .join(", ");
            format!("let data: [u8; {}] = [{}];", b.len(), byte_strings)
        }

        HexFormat::CStyleArray => {
            let byte_strings = b
                .iter()
                .map(|byte| format!("0x{:02X}", byte))
                .collect::<Vec<_>>()
                .join(", ");
            format!("unsigned char data[{}] = {{ {} }};", b.len(), byte_strings)
        }

        HexFormat::StringLiteral => {
            let byte_strings = b
                .iter()
                .map(|byte| format!("\\x{:02X}", byte))
                .collect::<Vec<_>>()
                .join("");
            format!("\"{}\"", byte_strings)
        }
    }
}
