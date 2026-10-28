use crate::{
    artifacts::*,
    diagnostics::{Diagnostic, DiagnosticCode},
    syntax::{Keyword, NodeKind, SyntaxNode, TokenKind},
};

use super::{
    super::{PendingProcess, UnitRegistry, units::identifier},
    error,
};

pub(super) fn check_header(node: &SyntaxNode, unit: &UnitRecord) -> Result<(), Box<Diagnostic>> {
    for token in node.all_tokens() {
        if matches!(
            token.kind(),
            TokenKind::Keyword(Keyword::Port | Keyword::Generic | Keyword::Generate)
        ) {
            return Err(error(
                token.span()?,
                DiagnosticCode::UnsupportedFeature,
                "ports, generics, and generate are outside initial unit analysis; coverage L005",
            ));
        }
    }
    let mut ending = None;
    let mut after_end = false;
    for token in node.all_tokens() {
        if token.kind() == TokenKind::Keyword(Keyword::End) {
            ending = None;
            after_end = true;
        } else if after_end && let Some(name) = identifier(&token) {
            ending = Some((name, token.span()?));
        }
    }
    let expected = match &unit.identity.kind {
        UnitKind::Architecture { name } => name,
        _ => &unit.identity.primary,
    };
    if let Some((ending, span)) = ending
        && ending.normalized() != expected.normalized()
    {
        let mut diagnostic = error(
            span,
            DiagnosticCode::NameMismatch,
            "ending identifier does not match the unit declaration",
        );
        if let Some(declaration) = unit.span {
            diagnostic
                .related
                .push(crate::diagnostics::DiagnosticLabel {
                    span: declaration,
                    message: "unit declared here".into(),
                });
        }
        return Err(diagnostic);
    }
    Ok(())
}

pub(super) fn processes(
    artifact: &mut ArtifactEnvelope,
    registry: &UnitRegistry,
    unit: &UnitRecord,
    node: &SyntaxNode,
) -> Result<(), Box<Diagnostic>> {
    let Some(part) = node.children().find(|child| {
        matches!(
            child.kind(),
            NodeKind::EntityStatementPart | NodeKind::ArchitectureStatementPart
        )
    }) else {
        return Ok(());
    };
    let scopes = registry
        .scopes()
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Lexical(unit.id))
        .collect::<Vec<_>>();
    let scoped_nodes = node
        .descendants()
        .filter(|child| {
            matches!(
                child.kind(),
                NodeKind::ProcessStatement | NodeKind::BlockStatement | NodeKind::SubprogramBody
            )
        })
        .collect::<Vec<_>>();
    if scopes.len() != scoped_nodes.len() {
        return Err(error(
            node.span()?,
            DiagnosticCode::ArtifactValidation,
            "name/syntax lexical scopes do not match",
        ));
    }
    let positions = scoped_nodes
        .iter()
        .zip(&scopes)
        .map(|(node, scope)| Ok((node.span()?, scope.id)))
        .collect::<Result<Vec<_>, Box<Diagnostic>>>()?;
    for statement in part.children() {
        if unit.identity.kind != UnitKind::Entity && statement.kind() == NodeKind::ProcessStatement
        {
            let span = statement.span()?;
            let scope = positions
                .iter()
                .find_map(|(position, scope)| (*position == span).then_some(*scope))
                .ok_or_else(|| {
                    error(
                        span,
                        DiagnosticCode::ArtifactValidation,
                        "process lexical scope is unavailable",
                    )
                })?;
            let mut tokens = statement.all_tokens();
            let name = match (tokens.next(), tokens.next()) {
                (Some(first), Some(second)) if second.text() == ":" => identifier(&first),
                _ => None,
            };
            let ArtifactPayload::Analysis(analysis) = &mut artifact.payload else {
                return Err(error(
                    span,
                    DiagnosticCode::ArtifactValidation,
                    "process context has wrong stage",
                ));
            };
            analysis
                .pending_processes
                .push(PendingProcess { name, scope, span });
        } else {
            return Err(error(
                statement.span()?,
                DiagnosticCode::UnsupportedFeature,
                "concurrent statement is outside initial architecture process boundaries; coverage L005",
            ));
        }
    }
    Ok(())
}
