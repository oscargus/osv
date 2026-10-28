use std::collections::BTreeMap;

use crate::{
    artifacts::*,
    diagnostics::{Diagnostic, DiagnosticCode},
    intrinsics::NativeType,
    syntax::{Keyword, NodeKind, SyntaxNode, TokenKind},
    values::{IntegerValue, PrimitiveValue, TimeValue},
};

use super::{
    super::{
        ExpressionOptions, ObjectDeclaration, ObjectInitialization, analyze_expression,
        constant::FoldResult, expressions::native_type, names::ScopeLookup, units::identifier,
    },
    error,
};

fn default_value(native: NativeType) -> Option<PrimitiveValue> {
    match native {
        NativeType::Boolean => Some(PrimitiveValue::Boolean(false)),
        NativeType::Integer => Some(PrimitiveValue::Integer(IntegerValue(i64::MIN))),
        NativeType::Time => Some(PrimitiveValue::Time(TimeValue(i64::MIN))),
        NativeType::Natural | NativeType::DelayLength => {
            Some(if native == NativeType::DelayLength {
                PrimitiveValue::Time(TimeValue(0))
            } else {
                PrimitiveValue::Integer(IntegerValue(0))
            })
        }
        NativeType::Positive => Some(PrimitiveValue::Integer(IntegerValue(1))),
        _ => None,
    }
}

pub(super) fn analyze(
    artifact: &mut ArtifactEnvelope,
    unit: &UnitRecord,
    node: &SyntaxNode,
    constants: &mut BTreeMap<SymbolId, PrimitiveValue>,
) -> Result<(), Box<Diagnostic>> {
    let node_span = node.span()?;
    let scope = artifact
        .scopes
        .iter()
        .find_map(|scope| (scope.kind == ScopeKind::Unit(unit.id)).then_some(scope.id))
        .ok_or_else(|| {
            error(
                node_span,
                DiagnosticCode::MissingUnit,
                "unit scope is unavailable",
            )
        })?;
    let part = node.children().find(|child| {
        matches!(
            child.kind(),
            NodeKind::EntityDeclarativePart | NodeKind::ArchitectureDeclarativePart
        )
    });
    let Some(part) = part else {
        return Ok(());
    };
    for declaration in part.children() {
        if !matches!(
            declaration.kind(),
            NodeKind::ConstantDeclaration | NodeKind::SignalDeclaration
        ) {
            return Err(error(
                declaration.span()?,
                DiagnosticCode::UnsupportedFeature,
                "declaration is outside initial scalar constant/signal analysis; coverage L007/L009",
            ));
        }
        let span = declaration.span()?;
        if declaration.all_tokens().any(|token| {
            matches!(
                token.kind(),
                TokenKind::Keyword(Keyword::Bus | Keyword::Register)
            )
        }) {
            return Err(error(
                span,
                DiagnosticCode::UnsupportedFeature,
                "guarded bus/register signals require later driver semantics; coverage L005",
            ));
        }
        let subtype = declaration
            .children()
            .find(|child| child.kind() == NodeKind::SubtypeIndication)
            .ok_or_else(|| {
                error(
                    span,
                    DiagnosticCode::TypeMismatch,
                    "object has no subtype indication",
                )
            })?;
        let subtype_span = subtype.span()?;
        let tokens = subtype.all_tokens().collect::<Vec<_>>();
        if tokens.len() != 1 || tokens[0].kind() != TokenKind::Identifier {
            return Err(error(
                subtype.span()?,
                DiagnosticCode::UnsupportedFeature,
                "only simple installed scalar subtype names are supported; coverage L009",
            ));
        }
        let name = identifier(&tokens[0]).ok_or_else(|| {
            error(
                subtype_span,
                DiagnosticCode::MissingName,
                "object subtype has no type name",
            )
        })?;
        let lookup = ScopeLookup::new(artifact).map_err(|failure| {
            error(
                span,
                DiagnosticCode::ArtifactValidation,
                failure.to_string(),
            )
        })?;
        let candidates = lookup
            .lookup_with_standard(scope, &name)
            .map_err(|failure| error(span, DiagnosticCode::MissingName, failure.to_string()))?;
        if candidates.len() != 1 || candidates[0].kind != SymbolKind::Type {
            return Err(error(
                subtype.span()?,
                DiagnosticCode::MissingName,
                "object subtype does not identify a visible type declaration",
            ));
        }
        let type_id = candidates[0].type_id.ok_or_else(|| {
            error(
                span,
                DiagnosticCode::TypeMismatch,
                "type declaration has no type identity",
            )
        })?;
        let native = native_type(artifact, type_id).map_err(|failure| {
            error(
                span,
                DiagnosticCode::UnsupportedFeature,
                failure.to_string(),
            )
        })?;
        let default = default_value(native).ok_or_else(|| {
            error(
                subtype_span,
                DiagnosticCode::UnsupportedFeature,
                "object type is outside the initial boolean/integer/time subset; coverage L007",
            )
        })?;
        let kind = if declaration.kind() == NodeKind::ConstantDeclaration {
            SymbolKind::Constant
        } else {
            SymbolKind::Signal
        };
        let explicit = declaration
            .all_tokens()
            .any(|token| token.kind() == TokenKind::ColonEq);
        let (initialization, value) = if explicit {
            let initial = declaration
                .children()
                .find(|child| child.kind() == NodeKind::InitialValue)
                .ok_or_else(|| {
                    error(
                        span,
                        DiagnosticCode::MissingInitializer,
                        "object initializer is unavailable",
                    )
                })?;
            let expression = initial.children().next().ok_or_else(|| {
                error(
                    span,
                    DiagnosticCode::MissingInitializer,
                    "object initializer has no expression",
                )
            })?;
            let analyzed = analyze_expression(
                &expression,
                artifact,
                scope,
                Some(type_id),
                constants,
                ExpressionOptions::default(),
            )
            .map_err(|mut diagnostics| Box::new(diagnostics.remove(0)))?;
            (
                ObjectInitialization::Expression(analyzed.plan),
                match analyzed.constant {
                    FoldResult::Value(value) => Some(value),
                    FoldResult::NotYetStatic => None,
                },
            )
        } else {
            if kind == SymbolKind::Constant {
                return Err(error(
                    span,
                    DiagnosticCode::MissingInitializer,
                    "constant in an entity/architecture requires an initializer",
                ));
            }
            (ObjectInitialization::Default(default), None)
        };
        let names = declaration
            .all_tokens()
            .take_while(|token| token.kind() != TokenKind::Colon)
            .filter_map(|token| identifier(&token).map(|name| (token, name)))
            .collect::<Vec<_>>();
        for (token, name) in names {
            let id = artifact
                .symbols
                .iter()
                .map(|symbol| symbol.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .map(SymbolId)
                .ok_or_else(|| {
                    error(
                        span,
                        DiagnosticCode::UnsupportedFeature,
                        "object symbol ID resource limit",
                    )
                })?;
            artifact.symbols.push(SymbolRecord {
                id,
                unit: unit.id,
                scope,
                name,
                kind,
                type_id: Some(type_id),
                span: Some(token.span()?),
                native: None,
            });
            let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
                return Err(error(
                    span,
                    DiagnosticCode::ArtifactValidation,
                    "object context has wrong artifact stage",
                ));
            };
            analysis.declarations.push(id);
            analysis.objects.push(ObjectDeclaration {
                symbol: id,
                initialization: initialization.clone(),
            });
            if kind == SymbolKind::Constant
                && let Some(value) = value
            {
                constants.insert(id, value);
            }
        }
    }
    Ok(())
}
