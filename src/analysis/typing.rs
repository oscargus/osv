use std::collections::BTreeMap;

use num_bigint::{BigInt, BigUint};

use crate::{
    artifacts::{ArtifactEnvelope, Identifier, ScopeId, SymbolId, SymbolKind, TypeId},
    diagnostics::{Diagnostic, DiagnosticCode, DiagnosticNote, DiagnosticStage, Severity},
    intrinsics::{IntrinsicReference, IntrinsicRegistry, NativeBinding, NativeType, Operator},
    source::ByteSpan,
    syntax::{Keyword, NodeKind, SyntaxNode, SyntaxToken, TokenKind},
    values::{PrimitiveValue, TimeValue},
};

use super::{
    constant::FoldResult,
    expressions::{ExpressionId, ExpressionKind, ExpressionPlan, TypedExpression, native_type},
    literals::{self, LiteralError},
    names::ScopeLookup,
};

#[derive(Clone, Copy, Debug)]
pub struct ExpressionOptions {
    pub fold_constants: bool,
}

impl Default for ExpressionOptions {
    fn default() -> Self {
        Self {
            fold_constants: true,
        }
    }
}

pub struct AnalyzedExpression {
    pub plan: ExpressionPlan,
    pub constant: FoldResult,
}

fn diagnostic(code: DiagnosticCode, span: ByteSpan, message: impl Into<String>) -> Box<Diagnostic> {
    Box::new(
        Diagnostic::new(code, DiagnosticStage::Analysis, Severity::Error, message)
            .with_primary(span, "expression is invalid here"),
    )
}

fn literal_error(error: LiteralError, span: ByteSpan) -> Box<Diagnostic> {
    let code = match error {
        LiteralError::UnsupportedReal | LiteralError::ResourceLimit => {
            DiagnosticCode::UnsupportedFeature
        }
        _ => DiagnosticCode::LiteralRange,
    };
    let mut result = diagnostic(code, span, error.to_string());
    if code == DiagnosticCode::UnsupportedFeature {
        result.notes.push(DiagnosticNote::Text("coverage L003/L007/L008: exact numeric resource limits are explicit; real/fractional physical support remains in 031".into()));
    }
    result
}

fn base(native: NativeType) -> NativeType {
    match native {
        NativeType::Natural | NativeType::Positive => NativeType::Integer,
        NativeType::DelayLength => NativeType::Time,
        _ => native,
    }
}

fn operator(kind: TokenKind) -> Option<Operator> {
    match kind {
        TokenKind::Plus => Some(Operator::Add),
        TokenKind::Minus => Some(Operator::Subtract),
        TokenKind::Times => Some(Operator::Multiply),
        TokenKind::Div => Some(Operator::Divide),
        TokenKind::EQ => Some(Operator::Equal),
        TokenKind::NE => Some(Operator::NotEqual),
        TokenKind::LT => Some(Operator::Less),
        TokenKind::LTE => Some(Operator::LessEqual),
        TokenKind::GT => Some(Operator::Greater),
        TokenKind::GTE => Some(Operator::GreaterEqual),
        TokenKind::Keyword(Keyword::And) => Some(Operator::And),
        TokenKind::Keyword(Keyword::Or) => Some(Operator::Or),
        TokenKind::Keyword(Keyword::Not) => Some(Operator::Not),
        _ => None,
    }
}

fn expression_kind(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::BinaryExpression
            | NodeKind::UnaryExpression
            | NodeKind::LiteralExpression
            | NodeKind::PhysicalLiteralExpression
            | NodeKind::NameExpression
            | NodeKind::ParenthesizedExpression
            | NodeKind::ParenthesizedExpressionOrAggregate
    )
}

pub fn analyze_expression(
    node: &SyntaxNode,
    artifact: &ArtifactEnvelope,
    scope: ScopeId,
    expected: Option<TypeId>,
    constants: &BTreeMap<SymbolId, PrimitiveValue>,
    options: ExpressionOptions,
) -> Result<AnalyzedExpression, Vec<Diagnostic>> {
    let span = node.span().map_err(|error| vec![*error])?;
    let lookup = ScopeLookup::new(artifact).map_err(|error| {
        vec![*diagnostic(
            DiagnosticCode::ArtifactValidation,
            span,
            error.to_string(),
        )]
    })?;
    let mut builder = Builder {
        artifact,
        lookup,
        scope,
        constants,
        options,
        registry: IntrinsicRegistry::standard(),
        nodes: Vec::new(),
        depth: 0,
    };
    let mut result = builder.build(node).map_err(|error| vec![*error])?;
    if let Some(expected) = expected {
        let actual = builder.nodes[usize::try_from(result.0).map_err(|_| {
            vec![*diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "expression ID resource limit",
            )]
        })?]
        .type_id;
        let expected_native = native_type(artifact, expected).map_err(|error| {
            vec![*diagnostic(
                DiagnosticCode::TypeMismatch,
                span,
                error.to_string(),
            )]
        })?;
        let actual_native = native_type(artifact, actual).map_err(|error| {
            vec![*diagnostic(
                DiagnosticCode::TypeMismatch,
                span,
                error.to_string(),
            )]
        })?;
        if base(expected_native) != base(actual_native) {
            return Err(vec![
                Diagnostic::new(
                    DiagnosticCode::TypeMismatch,
                    DiagnosticStage::Analysis,
                    Severity::Error,
                    "expression does not match the expected type",
                )
                .with_primary(
                    span,
                    format!("expression has type {}", actual_native.name()),
                )
                .with_note(DiagnosticNote::ExpectedActual {
                    expected: expected_native.name().into(),
                    actual: actual_native.name().into(),
                }),
            ]);
        }
        let root = builder.nodes.last_mut().ok_or_else(|| {
            vec![*diagnostic(
                DiagnosticCode::Syntax,
                span,
                "expression has no result",
            )]
        })?;
        root.type_id = expected;
        result = root.id;
    }
    let mut plan = ExpressionPlan {
        nodes: builder.nodes,
        result,
    };
    let constant = plan.evaluate(artifact, constants).map_err(|error| {
        vec![*diagnostic(
            DiagnosticCode::Arithmetic,
            span,
            error.to_string(),
        )]
    })?;
    if options.fold_constants
        && let FoldResult::Value(value) = constant
    {
        let root = plan
            .nodes
            .last()
            .ok_or_else(|| {
                vec![*diagnostic(
                    DiagnosticCode::Syntax,
                    span,
                    "expression has no result",
                )]
            })?
            .clone();
        plan.nodes = vec![TypedExpression {
            kind: ExpressionKind::Literal(value),
            ..root
        }];
    }
    plan.validate(artifact).map_err(|error| {
        vec![*diagnostic(
            DiagnosticCode::TypeMismatch,
            span,
            error.to_string(),
        )]
    })?;
    Ok(AnalyzedExpression { plan, constant })
}

struct Builder<'artifact, 'values> {
    artifact: &'artifact ArtifactEnvelope,
    lookup: ScopeLookup<'artifact>,
    scope: ScopeId,
    constants: &'values BTreeMap<SymbolId, PrimitiveValue>,
    options: ExpressionOptions,
    registry: IntrinsicRegistry,
    nodes: Vec<TypedExpression>,
    depth: usize,
}

impl Builder<'_, '_> {
    fn type_id(&self, native: NativeType, span: ByteSpan) -> Result<TypeId, Box<Diagnostic>> {
        self.artifact
            .symbols
            .iter()
            .find_map(|symbol| {
                (symbol.native == Some(NativeBinding::Type(native)))
                    .then_some(symbol.type_id)
                    .flatten()
            })
            .ok_or_else(|| {
                diagnostic(
                    DiagnosticCode::MissingName,
                    span,
                    format!("native type {} is not installed", native.name()),
                )
            })
    }

    fn push(
        &mut self,
        type_id: TypeId,
        span: ByteSpan,
        kind: ExpressionKind,
    ) -> Result<ExpressionId, Box<Diagnostic>> {
        if self.nodes.len() >= 65_536 {
            return Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "expression node resource limit",
            ));
        }
        let id = ExpressionId(u32::try_from(self.nodes.len()).map_err(|_| {
            diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "expression ID resource limit",
            )
        })?);
        self.nodes.push(TypedExpression {
            id,
            type_id,
            span,
            kind,
        });
        Ok(id)
    }

    fn build(&mut self, node: &SyntaxNode) -> Result<ExpressionId, Box<Diagnostic>> {
        let span = node.span()?;
        if self.depth >= 128 {
            return Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "expression depth resource limit",
            ));
        }
        self.depth += 1;
        let result = self.build_inner(node, span);
        self.depth -= 1;
        result
    }

    fn literal(
        &mut self,
        node: &SyntaxNode,
        span: ByteSpan,
        negative: bool,
    ) -> Result<ExpressionId, Box<Diagnostic>> {
        let token = node
            .all_tokens()
            .find(|token| token.kind() == TokenKind::AbstractLiteral)
            .ok_or_else(|| {
                diagnostic(
                    DiagnosticCode::UnsupportedFeature,
                    span,
                    "only boolean/integer/time literals are supported; coverage L003/L007",
                )
            })?;
        let magnitude =
            literals::integer_magnitude(&token).map_err(|error| literal_error(error, span))?;
        if node.kind() == NodeKind::PhysicalLiteralExpression {
            let unit = node
                .all_tokens()
                .find(|token| token.kind() == TokenKind::Identifier)
                .ok_or_else(|| {
                    diagnostic(
                        DiagnosticCode::MissingName,
                        span,
                        "physical literal has no unit",
                    )
                })?;
            let name = Identifier::Basic(unit.text()).normalized();
            let candidates = self
                .lookup
                .lookup_with_standard(self.scope, &name)
                .map_err(|error| {
                    diagnostic(DiagnosticCode::MissingName, span, error.to_string())
                })?;
            let units = candidates
                .iter()
                .filter_map(|symbol| match symbol.native {
                    Some(NativeBinding::PhysicalUnit { primary_units }) => Some(primary_units),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if units.len() != 1 {
                return Err(diagnostic(
                    DiagnosticCode::MissingName,
                    unit.span()?,
                    "physical unit is not a uniquely resolved native TIME unit",
                ));
            }
            let scaled = BigInt::from(magnitude) * BigInt::from(units[0]);
            let value = i64::try_from(if negative { -scaled } else { scaled }).map_err(|_| {
                diagnostic(
                    DiagnosticCode::LiteralRange,
                    span,
                    "physical literal is outside the TIME base range",
                )
            })?;
            self.push(
                self.type_id(NativeType::Time, span)?,
                span,
                ExpressionKind::Literal(PrimitiveValue::Time(TimeValue(value))),
            )
        } else {
            let value = literals::checked_integer(magnitude, negative).map_err(|error| {
                diagnostic(DiagnosticCode::LiteralRange, span, error.to_string())
            })?;
            self.push(
                self.type_id(NativeType::Integer, span)?,
                span,
                ExpressionKind::Literal(PrimitiveValue::Integer(value)),
            )
        }
    }

    fn build_inner(
        &mut self,
        node: &SyntaxNode,
        span: ByteSpan,
    ) -> Result<ExpressionId, Box<Diagnostic>> {
        match node.kind() {
            NodeKind::LiteralExpression | NodeKind::PhysicalLiteralExpression => {
                self.literal(node, span, false)
            }
            NodeKind::ParenthesizedExpression | NodeKind::ParenthesizedExpressionOrAggregate => {
                let direct = node.children().find(|child| expression_kind(child.kind()));
                let child = if let Some(child) = direct {
                    child
                } else {
                    let list = node
                        .children()
                        .find(|child| child.kind() == NodeKind::ElementAssociationList)
                        .ok_or_else(|| {
                            diagnostic(
                                DiagnosticCode::UnsupportedFeature,
                                span,
                                "aggregate expressions are not implemented; coverage L010",
                            )
                        })?;
                    let elements = list
                        .children()
                        .filter(|child| child.kind() == NodeKind::ElementAssociation)
                        .collect::<Vec<_>>();
                    if elements.len() != 1
                        || elements[0]
                            .children()
                            .any(|child| child.kind() == NodeKind::ElementChoices)
                    {
                        return Err(diagnostic(
                            DiagnosticCode::UnsupportedFeature,
                            span,
                            "aggregate expressions are not implemented; coverage L010",
                        ));
                    }
                    elements[0]
                        .children()
                        .find(|child| expression_kind(child.kind()))
                        .ok_or_else(|| {
                            diagnostic(
                                DiagnosticCode::UnsupportedFeature,
                                span,
                                "unsupported parenthesized expression",
                            )
                        })?
                };
                self.build(&child)
            }
            NodeKind::NameExpression => self.name(node, span),
            NodeKind::UnaryExpression => {
                let token = node.tokens().next().ok_or_else(|| {
                    diagnostic(
                        DiagnosticCode::Syntax,
                        span,
                        "unary expression has no operator",
                    )
                })?;
                let token_span = token.span()?;
                let operation = operator(token.kind()).ok_or_else(|| {
                    diagnostic(
                        DiagnosticCode::UnsupportedFeature,
                        token_span,
                        "unary operator is outside the initial subset; coverage L007",
                    )
                })?;
                let child = node
                    .children()
                    .find(|child| expression_kind(child.kind()))
                    .ok_or_else(|| {
                        diagnostic(
                            DiagnosticCode::Syntax,
                            span,
                            "unary expression has no operand",
                        )
                    })?;
                if operation == Operator::Subtract
                    && matches!(
                        child.kind(),
                        NodeKind::LiteralExpression | NodeKind::PhysicalLiteralExpression
                    )
                {
                    if child.kind() == NodeKind::PhysicalLiteralExpression {
                        self.select(operation, &[NativeType::Time], span, &[])?;
                        return self.literal(&child, span, true);
                    }
                    let token = child
                        .all_tokens()
                        .find(|token| token.kind() == TokenKind::AbstractLiteral)
                        .ok_or_else(|| {
                            diagnostic(
                                DiagnosticCode::UnsupportedFeature,
                                span,
                                "unsupported unary literal",
                            )
                        })?;
                    let magnitude = literals::integer_magnitude(&token)
                        .map_err(|error| literal_error(error, span))?;
                    if child.kind() == NodeKind::LiteralExpression
                        && magnitude > BigUint::from(i64::MAX.unsigned_abs())
                    {
                        self.select(operation, &[NativeType::Integer], span, &[])?;
                        return self.literal(&child, span, true);
                    }
                }
                let operand = self.build(&child)?;
                self.call(operation, vec![operand], span)
            }
            NodeKind::BinaryExpression => {
                let children = node
                    .children()
                    .filter(|child| expression_kind(child.kind()))
                    .collect::<Vec<_>>();
                if children.len() != 2 {
                    return Err(diagnostic(
                        DiagnosticCode::Syntax,
                        span,
                        "binary expression has invalid operand structure",
                    ));
                }
                let token = node.tokens().next().ok_or_else(|| {
                    diagnostic(
                        DiagnosticCode::Syntax,
                        span,
                        "binary expression has no operator",
                    )
                })?;
                let token_span = token.span()?;
                let operation = operator(token.kind()).ok_or_else(|| {
                    diagnostic(
                        DiagnosticCode::UnsupportedFeature,
                        token_span,
                        "operator is outside the initial subset; coverage L007",
                    )
                })?;
                let left = self.build(&children[0])?;
                let right = self.build(&children[1])?;
                self.call(operation, vec![left, right], span)
            }
            _ => Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "expression form is not supported in the initial subset; coverage L007",
            )),
        }
    }

    fn name(&mut self, node: &SyntaxNode, span: ByteSpan) -> Result<ExpressionId, Box<Diagnostic>> {
        let tokens = node
            .all_tokens()
            .filter(|token| token.kind() == TokenKind::Identifier)
            .collect::<Vec<SyntaxToken>>();
        if tokens.len() != 1 {
            return Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "selected/call/attribute names require later resolution; coverage L006",
            ));
        }
        let text = tokens[0].text();
        let name = if let Some(inner) = text
            .strip_prefix('\\')
            .and_then(|text| text.strip_suffix('\\'))
        {
            Identifier::Extended(inner.replace("\\\\", "\\"))
        } else {
            Identifier::Basic(text).normalized()
        };
        let candidates = self
            .lookup
            .lookup_with_standard(self.scope, &name)
            .map_err(|error| diagnostic(DiagnosticCode::MissingName, span, error.to_string()))?;
        if candidates.is_empty() {
            return Err(diagnostic(
                DiagnosticCode::MissingName,
                span,
                "name has no visible declaration",
            ));
        }
        if candidates.len() != 1 {
            return Err(diagnostic(
                DiagnosticCode::AmbiguousOverload,
                span,
                "name does not select one declaration in the initial subset",
            ));
        }
        let symbol = candidates[0];
        let type_id = symbol.type_id.ok_or_else(|| {
            diagnostic(
                DiagnosticCode::TypeMismatch,
                span,
                "resolved declaration has no value type",
            )
        })?;
        match &symbol.native {
            Some(NativeBinding::Literal {
                type_id: NativeType::Boolean,
                position,
            }) => self.push(
                type_id,
                span,
                ExpressionKind::Literal(PrimitiveValue::Boolean(*position == 1)),
            ),
            Some(NativeBinding::PhysicalUnit { primary_units }) => self.push(
                type_id,
                span,
                ExpressionKind::Literal(PrimitiveValue::Time(TimeValue(*primary_units))),
            ),
            Some(NativeBinding::Subprogram(reference)) => {
                let definition = self
                    .registry
                    .definition(reference)
                    .map_err(|error| {
                        diagnostic(
                            DiagnosticCode::ArtifactCompatibility,
                            span,
                            error.to_string(),
                        )
                    })?
                    .ok_or_else(|| {
                        diagnostic(
                            DiagnosticCode::ArtifactCompatibility,
                            span,
                            "invalid native function reference",
                        )
                    })?;
                if !definition.signature.arguments.is_empty() {
                    return Err(diagnostic(
                        DiagnosticCode::UnsupportedFeature,
                        span,
                        "subprogram requires argument association; coverage L034",
                    ));
                }
                self.push(
                    type_id,
                    span,
                    ExpressionKind::Intrinsic {
                        reference: Box::new(reference.clone()),
                        arguments: Vec::new(),
                    },
                )
            }
            _ if matches!(
                symbol.kind,
                SymbolKind::Constant | SymbolKind::Signal | SymbolKind::Variable
            ) =>
            {
                self.push(type_id, span, ExpressionKind::Name(symbol.id))
            }
            _ => Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "resolved declaration is not a supported scalar value or native call; coverage L007",
            )),
        }
    }

    fn select(
        &self,
        operation: Operator,
        arguments: &[NativeType],
        span: ByteSpan,
        operands: &[ExpressionId],
    ) -> Result<IntrinsicReference, Box<Diagnostic>> {
        let candidates = self
            .lookup
            .lookup_with_standard(self.scope, &Identifier::Operator(operation))
            .map_err(|error| diagnostic(DiagnosticCode::MissingName, span, error.to_string()))?;
        if candidates.iter().any(|symbol| symbol.native.is_none()) {
            return Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "user-defined operator requires ordinary subprogram semantics; coverage L034/L037",
            ));
        }
        let matches = candidates
            .iter()
            .filter_map(|symbol| match &symbol.native {
                Some(NativeBinding::Subprogram(reference))
                    if reference.declaration.signature.arguments.len() == arguments.len()
                        && reference
                            .declaration
                            .signature
                            .arguments
                            .iter()
                            .zip(arguments)
                            .all(|(expected, actual)| base(*expected) == base(*actual)) =>
                {
                    Some(reference.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            return Ok(matches[0].clone());
        }
        if matches.len() > 1 {
            return Err(diagnostic(
                DiagnosticCode::AmbiguousOverload,
                span,
                "operator selects multiple compatible native profiles",
            ));
        }
        let bases = arguments
            .iter()
            .map(|native| base(*native))
            .collect::<Vec<_>>();
        let deferred_family = match (operation, bases.as_slice()) {
            (Operator::Add | Operator::Subtract, [NativeType::Integer | NativeType::Time]) => true,
            (
                Operator::Multiply,
                [NativeType::Time, NativeType::Integer] | [NativeType::Integer, NativeType::Time],
            ) => true,
            (
                Operator::Divide,
                [NativeType::Time, NativeType::Integer] | [NativeType::Time, NativeType::Time],
            ) => true,
            (
                Operator::Equal
                | Operator::NotEqual
                | Operator::Less
                | Operator::LessEqual
                | Operator::Greater
                | Operator::GreaterEqual,
                [left, right],
            ) if left == right
                && matches!(
                    left,
                    NativeType::Boolean | NativeType::Integer | NativeType::Time
                ) =>
            {
                true
            }
            _ => false,
        };
        if deferred_family {
            return Err(diagnostic(
                DiagnosticCode::UnsupportedFeature,
                span,
                "this legal scalar operator family is not implemented in the initial subset; coverage L007/L008",
            ));
        }
        let actual = arguments
            .iter()
            .map(|native| native.name())
            .collect::<Vec<_>>()
            .join(", ");
        let mut error = Diagnostic::new(
            DiagnosticCode::TypeMismatch,
            DiagnosticStage::Analysis,
            Severity::Error,
            "operator has no unique compatible profile in the initial subset",
        )
        .with_primary(
            span,
            format!("operator {operation:?} cannot accept ({actual})"),
        )
        .with_note(DiagnosticNote::ExpectedActual {
            expected: "one installed native operator profile with compatible operand types".into(),
            actual,
        });
        for id in operands {
            let operand = &self.nodes[usize::try_from(id.0).map_err(|_| {
                diagnostic(
                    DiagnosticCode::UnsupportedFeature,
                    span,
                    "expression index limit",
                )
            })?];
            let native = native_type(self.artifact, operand.type_id).map_err(|error| {
                diagnostic(
                    DiagnosticCode::TypeMismatch,
                    operand.span,
                    error.to_string(),
                )
            })?;
            error = error.with_related(operand.span, format!("operand has type {}", native.name()));
        }
        Err(Box::new(error))
    }

    fn call(
        &mut self,
        operation: Operator,
        arguments: Vec<ExpressionId>,
        span: ByteSpan,
    ) -> Result<ExpressionId, Box<Diagnostic>> {
        let types = arguments
            .iter()
            .map(|id| {
                let node = &self.nodes[usize::try_from(id.0).map_err(|_| {
                    diagnostic(
                        DiagnosticCode::UnsupportedFeature,
                        span,
                        "expression index limit",
                    )
                })?];
                native_type(self.artifact, node.type_id).map_err(|error| {
                    diagnostic(DiagnosticCode::TypeMismatch, node.span, error.to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let reference = self.select(operation, &types, span, &arguments)?;
        let result = reference.declaration.signature.result.ok_or_else(|| {
            diagnostic(
                DiagnosticCode::TypeMismatch,
                span,
                "statement cannot be used as an expression",
            )
        })?;
        let type_id = self.type_id(result, span)?;
        let id = self.push(
            type_id,
            span,
            ExpressionKind::Intrinsic {
                reference: Box::new(reference),
                arguments,
            },
        )?;
        let plan = ExpressionPlan {
            nodes: self.nodes.clone(),
            result: id,
        };
        let folded = plan
            .evaluate(self.artifact, self.constants)
            .map_err(|error| diagnostic(DiagnosticCode::Arithmetic, span, error.to_string()))?;
        if self.options.fold_constants
            && let FoldResult::Value(value) = folded
        {
            self.nodes[usize::try_from(id.0).map_err(|_| {
                diagnostic(
                    DiagnosticCode::UnsupportedFeature,
                    span,
                    "expression index limit",
                )
            })?]
            .kind = ExpressionKind::Literal(value);
        }
        Ok(id)
    }
}
