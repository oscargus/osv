use serde::{Deserialize, Serialize};

use crate::{
    artifacts::{Identifier, ScopeId, SymbolId},
    source::ByteSpan,
    values::PrimitiveValue,
};

use super::expressions::ExpressionPlan;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ObjectInitialization {
    Default(PrimitiveValue),
    Expression(ExpressionPlan),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectDeclaration {
    pub symbol: SymbolId,
    pub initialization: ObjectInitialization,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PendingProcess {
    pub name: Option<Identifier>,
    pub scope: ScopeId,
    pub span: ByteSpan,
}
