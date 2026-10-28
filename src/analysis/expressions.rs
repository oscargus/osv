use std::{collections::BTreeMap, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    artifacts::{ArtifactEnvelope, SymbolId, TypeDefinition, TypeId},
    intrinsics::{IntrinsicReference, IntrinsicRegistry, NativeType, NativeTypeShape},
    source::ByteSpan,
    values::{PrimitiveValue, ValueError},
};

use super::constant::{self, EvaluationError, FoldResult};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ExpressionId(pub u32);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ExpressionKind {
    Literal(PrimitiveValue),
    Name(SymbolId),
    Intrinsic {
        reference: Box<IntrinsicReference>,
        arguments: Vec<ExpressionId>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypedExpression {
    pub id: ExpressionId,
    pub type_id: TypeId,
    pub span: ByteSpan,
    pub kind: ExpressionKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionPlan {
    pub nodes: Vec<TypedExpression>,
    pub result: ExpressionId,
}

#[derive(Debug)]
pub enum ExpressionError {
    Invalid(String),
    Evaluation(EvaluationError),
    Value(ValueError),
}

impl fmt::Display for ExpressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => formatter.write_str(message),
            Self::Evaluation(error) => error.fmt(formatter),
            Self::Value(error) => error.fmt(formatter),
        }
    }
}
impl Error for ExpressionError {}

pub fn native_type(artifact: &ArtifactEnvelope, id: TypeId) -> Result<NativeType, ExpressionError> {
    let record = artifact
        .types
        .iter()
        .find(|record| record.id == id)
        .ok_or_else(|| ExpressionError::Invalid(format!("missing expression type {}", id.0)))?;
    match record.definition {
        TypeDefinition::Native(native) => Ok(native),
        _ => Err(ExpressionError::Invalid(
            "expression type is outside the initial native scalar subset".into(),
        )),
    }
}

fn base(native: NativeType) -> NativeType {
    match native {
        NativeType::Natural | NativeType::Positive => NativeType::Integer,
        NativeType::DelayLength => NativeType::Time,
        _ => native,
    }
}

fn compatible(left: NativeType, right: NativeType) -> bool {
    base(left) == base(right)
}

fn check_value(value: PrimitiveValue, native: NativeType) -> Result<(), ExpressionError> {
    let registry = IntrinsicRegistry::standard();
    if let Some(definition) = registry
        .types()
        .iter()
        .find(|definition| definition.id == native)
        && let NativeTypeShape::Subtype {
            minimum, maximum, ..
        } = definition.shape
    {
        let magnitude = match value {
            PrimitiveValue::Integer(value) => value.0,
            PrimitiveValue::Time(value) => value.0,
            _ => {
                return Err(ExpressionError::Invalid(
                    "literal does not match subtype".into(),
                ));
            }
        };
        crate::values::IntegerValue(magnitude)
            .check_range(minimum, maximum)
            .map_err(ExpressionError::Value)?;
    }
    let matches = matches!(
        (value, base(native)),
        (PrimitiveValue::Boolean(_), NativeType::Boolean)
            | (PrimitiveValue::Integer(_), NativeType::Integer)
            | (PrimitiveValue::Time(_), NativeType::Time)
    );
    if !matches {
        return Err(ExpressionError::Invalid(
            "literal is not compatible with its expression type".into(),
        ));
    }
    Ok(())
}

impl ExpressionPlan {
    pub fn validate(&self, artifact: &ArtifactEnvelope) -> Result<(), ExpressionError> {
        artifact
            .validate(artifact.stage())
            .map_err(|error| ExpressionError::Invalid(error.to_string()))?;
        let mut previous = BTreeMap::new();
        let registry = IntrinsicRegistry::standard();
        for node in &self.nodes {
            if previous.contains_key(&node.id) {
                return Err(ExpressionError::Invalid("duplicate expression ID".into()));
            }
            let native = native_type(artifact, node.type_id)?;
            let source = artifact
                .sources
                .iter()
                .find(|source| source.file.id == node.span.source)
                .ok_or_else(|| ExpressionError::Invalid("missing expression source".into()))?;
            if node.span.start > node.span.end
                || source
                    .file
                    .bytes
                    .as_ref()
                    .is_some_and(|bytes| node.span.checked_range(bytes.len()).is_none())
            {
                return Err(ExpressionError::Invalid(
                    "invalid expression source span".into(),
                ));
            }
            match &node.kind {
                ExpressionKind::Literal(value) => check_value(*value, native)?,
                ExpressionKind::Name(id) => {
                    let symbol = artifact
                        .symbols
                        .iter()
                        .find(|symbol| symbol.id == *id)
                        .ok_or_else(|| {
                            ExpressionError::Invalid("missing expression declaration".into())
                        })?;
                    if symbol.type_id != Some(node.type_id) {
                        return Err(ExpressionError::Invalid(
                            "name type differs from resolved declaration".into(),
                        ));
                    }
                }
                ExpressionKind::Intrinsic {
                    reference,
                    arguments,
                } => {
                    let definition = registry
                        .definition(reference)
                        .map_err(|error| ExpressionError::Invalid(error.to_string()))?
                        .ok_or_else(|| {
                            ExpressionError::Invalid(
                                "invalid expression intrinsic reference".into(),
                            )
                        })?;
                    if definition
                        .signature
                        .result
                        .is_none_or(|result| !compatible(result, native))
                        || arguments.len() != definition.signature.arguments.len()
                    {
                        return Err(ExpressionError::Invalid(
                            "intrinsic expression profile differs from its type or arity".into(),
                        ));
                    }
                    for (argument, expected) in
                        arguments.iter().zip(&definition.signature.arguments)
                    {
                        let actual = previous.get(argument).copied().ok_or_else(|| {
                            ExpressionError::Invalid(
                                "expression argument must reference an earlier node".into(),
                            )
                        })?;
                        if !compatible(actual, *expected) {
                            return Err(ExpressionError::Invalid(
                                "intrinsic operand type mismatch".into(),
                            ));
                        }
                    }
                    if !artifact.native_requirements.iter().any(|required| {
                        required.library.normalized()
                            == reference.declaration.unit.library.normalized()
                            && required.package.normalized()
                                == reference.declaration.unit.primary.normalized()
                            && required.declarations == reference.declaration.declarations
                            && required.abi_version == reference.declaration.abi_version
                    }) {
                        return Err(ExpressionError::Invalid(
                            "missing expression native dependency requirement".into(),
                        ));
                    }
                }
            }
            previous.insert(node.id, native);
        }
        if !previous.contains_key(&self.result) {
            return Err(ExpressionError::Invalid("missing result expression".into()));
        }
        Ok(())
    }

    pub fn evaluate(
        &self,
        artifact: &ArtifactEnvelope,
        values: &BTreeMap<SymbolId, PrimitiveValue>,
    ) -> Result<FoldResult, ExpressionError> {
        self.validate(artifact)?;
        let mut results = BTreeMap::new();
        for node in &self.nodes {
            let result = match &node.kind {
                ExpressionKind::Literal(value) => FoldResult::Value(*value),
                ExpressionKind::Name(id) => {
                    let symbol = artifact
                        .symbols
                        .iter()
                        .find(|symbol| symbol.id == *id)
                        .ok_or_else(|| {
                            ExpressionError::Invalid("missing expression declaration".into())
                        })?;
                    if matches!(
                        symbol.kind,
                        crate::artifacts::SymbolKind::Constant
                            | crate::artifacts::SymbolKind::EnumerationLiteral
                            | crate::artifacts::SymbolKind::PhysicalUnit
                    ) {
                        values
                            .get(id)
                            .copied()
                            .map(FoldResult::Value)
                            .unwrap_or(FoldResult::NotYetStatic)
                    } else {
                        FoldResult::NotYetStatic
                    }
                }
                ExpressionKind::Intrinsic {
                    reference,
                    arguments,
                } => {
                    let inputs = arguments
                        .iter()
                        .map(|id| match results.get(id) {
                            Some(FoldResult::Value(value)) => Some(*value),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>();
                    match inputs {
                        Some(inputs) => constant::fold(reference, &inputs)
                            .map_err(ExpressionError::Evaluation)?,
                        None => FoldResult::NotYetStatic,
                    }
                }
            };
            if let FoldResult::Value(value) = result {
                check_value(value, native_type(artifact, node.type_id)?)?;
            }
            results.insert(node.id, result);
        }
        results
            .remove(&self.result)
            .ok_or_else(|| ExpressionError::Invalid("missing expression result".into()))
    }
}
