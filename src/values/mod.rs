mod scalar;
mod time;

use std::{error::Error, fmt};

use crate::artifacts::TypeId;

pub use scalar::{
    BitValue, EnumerationValue, IntegerValue, PhysicalValue, PrimitiveValue, RealValue,
};
pub use time::{SimulationTime, TimeResolution, TimeUnit, TimeValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueError {
    Overflow,
    ZeroDivisor,
    IntegerRange {
        value: i64,
        minimum: i64,
        maximum: i64,
    },
    EnumerationPosition {
        position: u32,
        cardinality: u32,
    },
    TypeMismatch {
        expected: TypeId,
        actual: TypeId,
    },
    NonFiniteReal,
    InvalidUnitScale,
    NegativeDelay,
    ResolutionLoss {
        femtoseconds: i64,
        resolution_fs: u64,
    },
}

impl fmt::Display for ValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => formatter.write_str("VHDL value overflow"),
            Self::ZeroDivisor => formatter.write_str("division by zero"),
            Self::IntegerRange {
                value,
                minimum,
                maximum,
            } => write!(
                formatter,
                "integer {value} is outside {minimum} to {maximum}"
            ),
            Self::EnumerationPosition {
                position,
                cardinality,
            } => write!(
                formatter,
                "enumeration position {position} is outside {cardinality} literals"
            ),
            Self::TypeMismatch { expected, actual } => write!(
                formatter,
                "expected type {}, found type {}",
                expected.0, actual.0
            ),
            Self::NonFiniteReal => formatter.write_str("scalar REAL must be finite"),
            Self::InvalidUnitScale => formatter
                .write_str("physical unit scale must be positive and within signed 64-bit range"),
            Self::NegativeDelay => formatter.write_str("scheduled delay cannot be negative"),
            Self::ResolutionLoss {
                femtoseconds,
                resolution_fs,
            } => write!(
                formatter,
                "time {femtoseconds} fs is not an exact multiple of resolution {resolution_fs} fs"
            ),
        }
    }
}

impl Error for ValueError {}
