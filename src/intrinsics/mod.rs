mod profiles;
mod types;

use serde::{Deserialize, Serialize};

use crate::artifacts::{
    ContentFingerprint, Identifier, NATIVE_ABI_VERSION, UnitIdentity, UnitKind,
};

pub use types::{
    NativeAttribute, NativeLiteral, NativeType, NativeTypeDefinition, NativeTypeShape, NativeUnit,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    And,
    Or,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Statement {
    Report,
    Assert,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IntrinsicName {
    Operator(Operator),
    Function(Identifier),
    Statement(Statement),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IntrinsicId {
    IntegerAdd,
    IntegerSubtract,
    IntegerMultiply,
    IntegerDivide,
    IntegerNegate,
    IntegerEqual,
    IntegerNotEqual,
    IntegerLess,
    IntegerLessEqual,
    IntegerGreater,
    IntegerGreaterEqual,
    BooleanNot,
    BooleanAnd,
    BooleanOr,
    BooleanEqual,
    BooleanNotEqual,
    TimeAdd,
    TimeSubtract,
    TimeEqual,
    TimeLess,
    Report,
    Assert,
    Now,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Purity {
    Pure,
    Impure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Specialization {
    Scalar,
    Diagnostic,
    SimulationTime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Effects {
    None,
    Report,
    Assert,
    ObserveTime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Folding {
    StaticArguments,
    Never,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Signature {
    pub name: IntrinsicName,
    pub arguments: Vec<NativeType>,
    pub result: Option<NativeType>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicDefinition {
    pub id: IntrinsicId,
    pub signature: Signature,
    pub purity: Purity,
    pub specialization: Specialization,
    pub effects: Effects,
    pub folding: Folding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationIdentity {
    pub unit: UnitIdentity,
    pub declarations: ContentFingerprint,
    pub abi_version: u32,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicReference {
    pub id: IntrinsicId,
    pub declaration: DeclarationIdentity,
}

pub struct IntrinsicRegistry {
    definitions: Vec<IntrinsicDefinition>,
    types: Vec<NativeTypeDefinition>,
    attributes: Vec<NativeAttribute>,
}

impl IntrinsicRegistry {
    pub fn standard() -> Self {
        Self {
            definitions: profiles::operations(),
            types: types::standard_types(),
            attributes: vec![NativeAttribute {
                name: Identifier::Basic("foreign".into()),
                type_id: NativeType::String,
            }],
        }
    }

    pub fn definitions(&self) -> &[IntrinsicDefinition] {
        &self.definitions
    }
    pub fn types(&self) -> &[NativeTypeDefinition] {
        &self.types
    }
    pub fn attributes(&self) -> &[NativeAttribute] {
        &self.attributes
    }

    pub fn declaration_fingerprint(&self) -> Result<ContentFingerprint, postcard::Error> {
        Ok(ContentFingerprint::of_bytes(&postcard::to_allocvec(&(
            1u32,
            NATIVE_ABI_VERSION,
            &self.types,
            &self.attributes,
            &self.definitions,
        ))?))
    }

    pub fn declaration(
        &self,
        id: IntrinsicId,
    ) -> Result<Option<DeclarationIdentity>, postcard::Error> {
        let Some(definition) = self
            .definitions
            .iter()
            .find(|definition| definition.id == id)
        else {
            return Ok(None);
        };
        Ok(Some(DeclarationIdentity {
            unit: UnitIdentity {
                library: Identifier::Basic("std".into()),
                primary: Identifier::Basic("standard".into()),
                kind: UnitKind::Package,
            },
            declarations: self.declaration_fingerprint()?,
            abi_version: NATIVE_ABI_VERSION,
            signature: definition.signature.clone(),
        }))
    }

    pub fn resolve(
        &self,
        declaration: &DeclarationIdentity,
    ) -> Result<Option<IntrinsicReference>, postcard::Error> {
        let mut declaration = declaration.clone();
        declaration.unit = declaration.unit.normalized();
        if let IntrinsicName::Function(name) = &mut declaration.signature.name {
            *name = name.normalized();
        }
        for definition in &self.definitions {
            if self.declaration(definition.id)?.as_ref() == Some(&declaration) {
                return Ok(Some(IntrinsicReference {
                    id: definition.id,
                    declaration: declaration.clone(),
                }));
            }
        }
        Ok(None)
    }

    pub fn definition(
        &self,
        reference: &IntrinsicReference,
    ) -> Result<Option<&IntrinsicDefinition>, postcard::Error> {
        if self
            .resolve(&reference.declaration)?
            .is_none_or(|resolved| resolved.id != reference.id)
        {
            return Ok(None);
        }
        Ok(self
            .definitions
            .iter()
            .find(|definition| definition.id == reference.id))
    }
}
