mod binary;
mod bounds;
mod logic;
mod scalar;
mod time;

use std::{error::Error, fmt};

use crate::artifacts::TypeId;

pub use binary::{MAX_VECTOR_BITS, PackedBinary, PackedStorage};
pub use bounds::{ArrayDimension, ArrayShape, Direction, IndexRange, RangeConstraint};
pub use logic::{LogicState, LogicStorage, LogicVector};
pub use scalar::{
    BitValue, EnumerationValue, IntegerValue, PhysicalValue, PrimitiveValue, RealValue,
};
pub use time::{SimulationTime, TimeResolution, TimeUnit, TimeValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueError {
    AllocationLimit,
    VectorWidth {
        width: u64,
        maximum: u64,
    },
    VectorValueTooWide {
        width: u64,
        bits: u64,
    },
    VectorEncoding(&'static str),
    WidthMismatch {
        expected: u64,
        actual: u64,
    },
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
    IndexBounds {
        index: i64,
        range: IndexRange,
    },
    StorageOffset {
        offset: u64,
        length: u64,
    },
    ArrayDimensions {
        expected: usize,
        actual: usize,
    },
    UnconstrainedArray,
}

impl fmt::Display for ValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationLimit => formatter.write_str("value storage allocation failed"),
            Self::VectorWidth { width, maximum } => write!(
                formatter,
                "vector width {width} exceeds storage limit {maximum}"
            ),
            Self::VectorValueTooWide { width, bits } => write!(
                formatter,
                "{bits}-bit magnitude does not fit vector width {width}"
            ),
            Self::VectorEncoding(message) => {
                write!(formatter, "invalid vector encoding: {message}")
            }
            Self::WidthMismatch { expected, actual } => {
                write!(formatter, "expected width {expected}, found {actual}")
            }
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
            Self::IndexBounds { index, range } => write!(
                formatter,
                "array index {index} is outside {} {:?} {}",
                range.left, range.direction, range.right
            ),
            Self::StorageOffset { offset, length } => write!(
                formatter,
                "storage offset {offset} is outside array length {length}"
            ),
            Self::ArrayDimensions { expected, actual } => write!(
                formatter,
                "expected {expected} array dimensions, found {actual}"
            ),
            Self::UnconstrainedArray => formatter.write_str("array bounds are not constrained"),
        }
    }
}

impl Error for ValueError {}
