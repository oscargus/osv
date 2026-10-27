use serde::{Deserialize, Serialize};

use crate::{artifacts::Identifier, values::TimeUnit};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum NativeType {
    Boolean,
    Bit,
    Integer,
    Real,
    Time,
    Severity,
    Character,
    String,
    Natural,
    Positive,
    DelayLength,
    BooleanVector,
    BitVector,
    IntegerVector,
    RealVector,
    TimeVector,
    FileOpenKind,
    FileOpenStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NativeLiteral {
    Identifier(Identifier),
    Character(u8),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeUnit {
    pub name: Identifier,
    pub primary_units: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NativeTypeShape {
    Enumeration {
        literals: Vec<NativeLiteral>,
    },
    Integer {
        minimum: i64,
        maximum: i64,
    },
    Real {
        minimum_bits: u64,
        maximum_bits: u64,
    },
    Physical {
        minimum: i64,
        maximum: i64,
        units: Vec<NativeUnit>,
    },
    Subtype {
        base: NativeType,
        minimum: i64,
        maximum: i64,
    },
    Array {
        index: NativeType,
        element: NativeType,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeTypeDefinition {
    pub id: NativeType,
    pub name: Identifier,
    pub shape: NativeTypeShape,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAttribute {
    pub name: Identifier,
    pub type_id: NativeType,
}

impl NativeType {
    pub fn name(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Bit => "bit",
            Self::Integer => "integer",
            Self::Real => "real",
            Self::Time => "time",
            Self::Severity => "severity_level",
            Self::Character => "character",
            Self::String => "string",
            Self::Natural => "natural",
            Self::Positive => "positive",
            Self::DelayLength => "delay_length",
            Self::BooleanVector => "boolean_vector",
            Self::BitVector => "bit_vector",
            Self::IntegerVector => "integer_vector",
            Self::RealVector => "real_vector",
            Self::TimeVector => "time_vector",
            Self::FileOpenKind => "file_open_kind",
            Self::FileOpenStatus => "file_open_status",
        }
    }
}

fn enumeration(names: &[&str]) -> NativeTypeShape {
    NativeTypeShape::Enumeration {
        literals: names
            .iter()
            .map(|name| NativeLiteral::Identifier(Identifier::Basic((*name).into())))
            .collect(),
    }
}

pub(super) fn standard_types() -> Vec<NativeTypeDefinition> {
    use NativeType::*;
    let controls = [
        "nul", "soh", "stx", "etx", "eot", "enq", "ack", "bel", "bs", "ht", "lf", "vt", "ff", "cr",
        "so", "si", "dle", "dc1", "dc2", "dc3", "dc4", "nak", "syn", "etb", "can", "em", "sub",
        "esc", "fsp", "gsp", "rsp", "usp",
    ];
    let characters = (0u8..=255)
        .map(|character| match character {
            0..=31 => NativeLiteral::Identifier(Identifier::Basic(
                controls[usize::from(character)].into(),
            )),
            127 => NativeLiteral::Identifier(Identifier::Basic("del".into())),
            128..=159 => NativeLiteral::Identifier(Identifier::Basic(format!("c{character}"))),
            _ => NativeLiteral::Character(character),
        })
        .collect();
    let units = [
        ("fs", TimeUnit::Femtosecond),
        ("ps", TimeUnit::Picosecond),
        ("ns", TimeUnit::Nanosecond),
        ("us", TimeUnit::Microsecond),
        ("ms", TimeUnit::Millisecond),
        ("sec", TimeUnit::Second),
        ("min", TimeUnit::Minute),
        ("hr", TimeUnit::Hour),
    ]
    .into_iter()
    .map(|(name, unit)| NativeUnit {
        name: Identifier::Basic(name.into()),
        primary_units: unit.femtoseconds(),
    })
    .collect();
    vec![
        (Boolean, enumeration(&["false", "true"])),
        (
            Bit,
            NativeTypeShape::Enumeration {
                literals: vec![
                    NativeLiteral::Character(b'0'),
                    NativeLiteral::Character(b'1'),
                ],
            },
        ),
        (
            Integer,
            NativeTypeShape::Integer {
                minimum: i64::MIN,
                maximum: i64::MAX,
            },
        ),
        (
            Real,
            NativeTypeShape::Real {
                minimum_bits: f64::MIN.to_bits(),
                maximum_bits: f64::MAX.to_bits(),
            },
        ),
        (
            Time,
            NativeTypeShape::Physical {
                minimum: i64::MIN,
                maximum: i64::MAX,
                units,
            },
        ),
        (
            Severity,
            enumeration(&["note", "warning", "error", "failure"]),
        ),
        (
            Character,
            NativeTypeShape::Enumeration {
                literals: characters,
            },
        ),
        (
            String,
            NativeTypeShape::Array {
                index: Positive,
                element: Character,
            },
        ),
        (
            Natural,
            NativeTypeShape::Subtype {
                base: Integer,
                minimum: 0,
                maximum: i64::MAX,
            },
        ),
        (
            Positive,
            NativeTypeShape::Subtype {
                base: Integer,
                minimum: 1,
                maximum: i64::MAX,
            },
        ),
        (
            DelayLength,
            NativeTypeShape::Subtype {
                base: Time,
                minimum: 0,
                maximum: i64::MAX,
            },
        ),
        (
            BooleanVector,
            NativeTypeShape::Array {
                index: Natural,
                element: Boolean,
            },
        ),
        (
            BitVector,
            NativeTypeShape::Array {
                index: Natural,
                element: Bit,
            },
        ),
        (
            IntegerVector,
            NativeTypeShape::Array {
                index: Natural,
                element: Integer,
            },
        ),
        (
            RealVector,
            NativeTypeShape::Array {
                index: Natural,
                element: Real,
            },
        ),
        (
            TimeVector,
            NativeTypeShape::Array {
                index: Natural,
                element: Time,
            },
        ),
        (
            FileOpenKind,
            enumeration(&["read_mode", "write_mode", "append_mode"]),
        ),
        (
            FileOpenStatus,
            enumeration(&["open_ok", "status_error", "name_error", "mode_error"]),
        ),
    ]
    .into_iter()
    .map(|(id, shape)| NativeTypeDefinition {
        id,
        name: Identifier::Basic(id.name().into()),
        shape,
    })
    .collect()
}
