use std::{error::Error, fmt};

use crate::{
    intrinsics::{Folding, IntrinsicId, IntrinsicReference, IntrinsicRegistry, NativeType},
    values::{IntegerValue, PrimitiveValue, TimeValue, ValueError},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FoldResult {
    Value(PrimitiveValue),
    NotYetStatic,
}

#[derive(Debug)]
pub enum EvaluationError {
    InvalidReference,
    Encoding(postcard::Error),
    Value(ValueError),
    UnsupportedIntrinsic(IntrinsicId),
    Arity { expected: usize, actual: usize },
    ArgumentType { index: usize, expected: NativeType },
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReference => formatter.write_str("invalid intrinsic reference"),
            Self::Encoding(error) => error.fmt(formatter),
            Self::Value(error) => error.fmt(formatter),
            Self::UnsupportedIntrinsic(id) => write!(
                formatter,
                "intrinsic {id:?} has no constant kernel in this subset"
            ),
            Self::Arity { expected, actual } => write!(
                formatter,
                "expected {expected} intrinsic arguments, found {actual}"
            ),
            Self::ArgumentType { index, expected } => write!(
                formatter,
                "intrinsic argument {index} does not have type {expected:?}"
            ),
        }
    }
}
impl Error for EvaluationError {}

pub fn fold(
    reference: &IntrinsicReference,
    arguments: &[PrimitiveValue],
) -> Result<FoldResult, EvaluationError> {
    let registry = IntrinsicRegistry::standard();
    let definition = registry
        .definition(reference)
        .map_err(EvaluationError::Encoding)?
        .ok_or(EvaluationError::InvalidReference)?;
    if arguments.len() != definition.signature.arguments.len() {
        return Err(EvaluationError::Arity {
            expected: definition.signature.arguments.len(),
            actual: arguments.len(),
        });
    }
    if definition.folding == Folding::Never {
        return Ok(FoldResult::NotYetStatic);
    }
    for (index, (value, expected)) in arguments
        .iter()
        .zip(&definition.signature.arguments)
        .enumerate()
    {
        let matches = matches!(
            (value, expected),
            (PrimitiveValue::Integer(_), NativeType::Integer)
                | (PrimitiveValue::Boolean(_), NativeType::Boolean)
                | (PrimitiveValue::Time(_), NativeType::Time)
        );
        if !matches {
            return Err(EvaluationError::ArgumentType {
                index,
                expected: *expected,
            });
        }
    }
    let result = match (reference.id, arguments) {
        (
            IntrinsicId::IntegerAdd,
            [
                PrimitiveValue::Integer(left),
                PrimitiveValue::Integer(right),
            ],
        ) => PrimitiveValue::Integer(left.checked_add(*right).map_err(EvaluationError::Value)?),
        (
            IntrinsicId::IntegerSubtract,
            [
                PrimitiveValue::Integer(left),
                PrimitiveValue::Integer(right),
            ],
        ) => PrimitiveValue::Integer(left.checked_sub(*right).map_err(EvaluationError::Value)?),
        (
            IntrinsicId::IntegerMultiply,
            [
                PrimitiveValue::Integer(left),
                PrimitiveValue::Integer(right),
            ],
        ) => PrimitiveValue::Integer(left.checked_mul(*right).map_err(EvaluationError::Value)?),
        (
            IntrinsicId::IntegerDivide,
            [
                PrimitiveValue::Integer(left),
                PrimitiveValue::Integer(right),
            ],
        ) => PrimitiveValue::Integer(left.checked_div(*right).map_err(EvaluationError::Value)?),
        (IntrinsicId::IntegerNegate, [PrimitiveValue::Integer(value)]) => {
            PrimitiveValue::Integer(value.checked_neg().map_err(EvaluationError::Value)?)
        }
        (
            id,
            [
                PrimitiveValue::Integer(left),
                PrimitiveValue::Integer(right),
            ],
        ) => PrimitiveValue::Boolean(match id {
            IntrinsicId::IntegerEqual => left.0 == right.0,
            IntrinsicId::IntegerNotEqual => left.0 != right.0,
            IntrinsicId::IntegerLess => left.0 < right.0,
            IntrinsicId::IntegerLessEqual => left.0 <= right.0,
            IntrinsicId::IntegerGreater => left.0 > right.0,
            IntrinsicId::IntegerGreaterEqual => left.0 >= right.0,
            _ => return Err(EvaluationError::UnsupportedIntrinsic(id)),
        }),
        (IntrinsicId::BooleanNot, [PrimitiveValue::Boolean(value)]) => {
            PrimitiveValue::Boolean(!value)
        }
        (
            id,
            [
                PrimitiveValue::Boolean(left),
                PrimitiveValue::Boolean(right),
            ],
        ) => PrimitiveValue::Boolean(match id {
            IntrinsicId::BooleanAnd => *left && *right,
            IntrinsicId::BooleanOr => *left || *right,
            IntrinsicId::BooleanEqual => left == right,
            IntrinsicId::BooleanNotEqual => left != right,
            _ => return Err(EvaluationError::UnsupportedIntrinsic(id)),
        }),
        (IntrinsicId::TimeAdd, [PrimitiveValue::Time(left), PrimitiveValue::Time(right)]) => {
            PrimitiveValue::Time(TimeValue(
                IntegerValue(left.0)
                    .checked_add(IntegerValue(right.0))
                    .map_err(EvaluationError::Value)?
                    .0,
            ))
        }
        (IntrinsicId::TimeSubtract, [PrimitiveValue::Time(left), PrimitiveValue::Time(right)]) => {
            PrimitiveValue::Time(TimeValue(
                IntegerValue(left.0)
                    .checked_sub(IntegerValue(right.0))
                    .map_err(EvaluationError::Value)?
                    .0,
            ))
        }
        (IntrinsicId::TimeEqual, [PrimitiveValue::Time(left), PrimitiveValue::Time(right)]) => {
            PrimitiveValue::Boolean(left.0 == right.0)
        }
        (IntrinsicId::TimeLess, [PrimitiveValue::Time(left), PrimitiveValue::Time(right)]) => {
            PrimitiveValue::Boolean(left.0 < right.0)
        }
        _ => return Err(EvaluationError::UnsupportedIntrinsic(reference.id)),
    };
    Ok(FoldResult::Value(result))
}
