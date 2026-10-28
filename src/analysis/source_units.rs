mod boundaries;
mod objects;

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    artifacts::*,
    diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity},
    intrinsics::install_standard,
    source::{ByteSpan, SourceMap},
    syntax::{self, NodeKind, SyntaxNode},
};

use super::UnitRegistry;

fn failure(
    span: Option<ByteSpan>,
    code: DiagnosticCode,
    message: impl Into<String>,
) -> Vec<Diagnostic> {
    let diagnostic = Diagnostic::new(code, DiagnosticStage::Analysis, Severity::Error, message);
    vec![match span {
        Some(span) => diagnostic.with_primary(span, "unit analysis failed here"),
        None => diagnostic,
    }]
}

fn error(span: ByteSpan, code: DiagnosticCode, message: impl Into<String>) -> Box<Diagnostic> {
    Box::new(
        Diagnostic::new(code, DiagnosticStage::Analysis, Severity::Error, message)
            .with_primary(span, "unit analysis failed here"),
    )
}

fn retain_context(artifact: &mut ArtifactEnvelope, unit: UnitId) {
    let mut units = BTreeSet::from([unit]);
    while let Some(parent) = artifact
        .units
        .iter()
        .filter(|record| units.contains(&record.id))
        .filter_map(|record| record.parent)
        .find(|parent| !units.contains(parent))
    {
        units.insert(parent);
    }
    for symbol in &artifact.symbols {
        units.insert(symbol.unit);
    }
    artifact.units.retain(|record| units.contains(&record.id));
    artifact.scopes.retain(|scope| match scope.kind {
        ScopeKind::Library(_) => true,
        ScopeKind::Unit(unit) | ScopeKind::Lexical(unit) => units.contains(&unit),
    });
    let mut sources = BTreeSet::new();
    for span in artifact
        .units
        .iter()
        .filter_map(|unit| unit.span)
        .chain(artifact.symbols.iter().filter_map(|symbol| symbol.span))
    {
        sources.insert(span.source);
    }
    if let ArtifactPayload::Analysis(analysis) = &artifact.payload {
        for process in &analysis.pending_processes {
            sources.insert(process.span.source);
        }
    }
    artifact
        .sources
        .retain(|record| sources.contains(&record.file.id));
}

pub fn analyze_units(
    sources: &SourceMap,
    library: Identifier,
) -> Result<Vec<ArtifactEnvelope>, Vec<Diagnostic>> {
    if sources.iter().len() == 0 {
        return Err(failure(
            None,
            DiagnosticCode::SourceUnavailable,
            "unit analysis requires source bytes",
        ));
    }
    let mut registry = UnitRegistry::new(library);
    let mut nodes = BTreeMap::<UnitId, SyntaxNode>::new();
    for source in sources.iter() {
        let parsed = syntax::parse(source)?;
        let ids = registry.register(&parsed)?;
        for (id, wrapper) in ids.into_iter().zip(parsed.design_units()) {
            let unit = wrapper.descendants().find(|node| matches!(node.kind(), NodeKind::EntityDeclaration | NodeKind::ArchitectureBody))
                .ok_or_else(|| failure(Some(parsed.span()), DiagnosticCode::UnsupportedFeature, "only entities and architectures are supported by initial unit analysis; coverage L005"))?;
            boundaries::check_header(
                &unit,
                registry
                    .units()
                    .iter()
                    .find(|unit| unit.id == id)
                    .ok_or_else(|| {
                        failure(
                            Some(parsed.span()),
                            DiagnosticCode::MissingUnit,
                            "registered unit header is unavailable",
                        )
                    })?,
            )
            .map_err(|error| vec![*error])?;
            nodes.insert(id, unit);
        }
    }
    let mut results = Vec::new();
    let mut analyzed = BTreeMap::<UnitId, ArtifactEnvelope>::new();
    let mut values = BTreeMap::new();
    for unit in registry.units() {
        let mut artifact = ArtifactEnvelope {
            schema_version: SCHEMA_VERSION,
            compiler_compatibility: COMPILER_COMPATIBILITY,
            compiler_version: env!("CARGO_PKG_VERSION").into(),
            language: LanguageRevision::Vhdl2008,
            identity: unit.identity.clone(),
            sources: sources
                .iter()
                .cloned()
                .map(SourceRecord::embedded)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    failure(
                        unit.span,
                        DiagnosticCode::SourceUnavailable,
                        error.to_string(),
                    )
                })?,
            units: registry.units().to_vec(),
            scopes: registry.scopes().to_vec(),
            types: Vec::new(),
            symbols: Vec::new(),
            dependencies: Vec::new(),
            native_requirements: Vec::new(),
            payload: ArtifactPayload::Analysis(AnalysisArtifact {
                unit: unit.id,
                declarations: Vec::new(),
                objects: Vec::new(),
                pending_processes: Vec::new(),
            }),
        };
        if let Some(parent) = unit.parent {
            let primary = analyzed.get(&parent).ok_or_else(|| {
                failure(
                    unit.span,
                    DiagnosticCode::MissingUnit,
                    "architecture primary is not analyzed",
                )
            })?;
            artifact.dependencies.push(DependencyRecord {
                identity: primary.identity.clone(),
                stage: ArtifactStage::Analysis,
                role: DependencyRole::Declaration,
                fingerprint: primary.fingerprint().map_err(|error| {
                    failure(
                        unit.span,
                        DiagnosticCode::ArtifactValidation,
                        error.to_string(),
                    )
                })?,
                schema_version: SCHEMA_VERSION,
                compiler_compatibility: COMPILER_COMPATIBILITY,
            });
        }
        install_standard(&mut artifact).map_err(|error| {
            failure(
                unit.span,
                DiagnosticCode::ArtifactValidation,
                error.to_string(),
            )
        })?;
        let mut constants = BTreeMap::new();
        if let Some(parent) = unit.parent {
            let primary = analyzed.get(&parent).ok_or_else(|| {
                failure(
                    unit.span,
                    DiagnosticCode::MissingUnit,
                    "architecture primary is not analyzed",
                )
            })?;
            for symbol in primary
                .symbols
                .iter()
                .filter(|symbol| symbol.native.is_none())
            {
                artifact.symbols.push(symbol.clone());
            }
            constants = values.get(&parent).cloned().unwrap_or_default();
        }
        let node = nodes.get(&unit.id).ok_or_else(|| {
            failure(
                unit.span,
                DiagnosticCode::MissingUnit,
                "unit syntax is unavailable",
            )
        })?;
        objects::analyze(&mut artifact, unit, node, &mut constants)
            .map_err(|error| vec![*error])?;
        boundaries::processes(&mut artifact, &registry, unit, node)
            .map_err(|error| vec![*error])?;
        retain_context(&mut artifact, unit.id);
        artifact
            .validate(ArtifactStage::Analysis)
            .map_err(|error| {
                failure(
                    unit.span,
                    DiagnosticCode::ArtifactValidation,
                    error.to_string(),
                )
            })?;
        values.insert(unit.id, constants);
        analyzed.insert(unit.id, artifact.clone());
        results.push(artifact);
    }
    Ok(results)
}
