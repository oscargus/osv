use std::collections::BTreeSet;

use crate::{
    analysis::{
        ObjectInitialization,
        expressions::{check_value, native_type},
    },
    source::ByteSpan,
};

use super::{ArtifactEnvelope, ArtifactError, ArtifactPayload, ScopeKind, SymbolKind, TableKind};

impl ArtifactEnvelope {
    fn unit_data_span(&self, span: ByteSpan) -> Result<(), ArtifactError> {
        let source = self
            .sources
            .iter()
            .find(|source| source.file.id == span.source)
            .ok_or(ArtifactError::DanglingReference {
                table: TableKind::Source,
                id: span.source.0,
            })?;
        if span.start > span.end
            || source
                .file
                .bytes
                .as_ref()
                .is_some_and(|bytes| span.checked_range(bytes.len()).is_none())
        {
            return Err(ArtifactError::InvalidSpan(span));
        }
        Ok(())
    }

    pub(super) fn validate_unit_data(&self) -> Result<(), ArtifactError> {
        let ArtifactPayload::Analysis(analysis) = &self.payload else {
            return Ok(());
        };
        let mut objects = BTreeSet::new();
        for object in &analysis.objects {
            let symbol = self
                .symbols
                .iter()
                .find(|symbol| symbol.id == object.symbol)
                .ok_or(ArtifactError::DanglingReference {
                    table: TableKind::Symbol,
                    id: object.symbol.0,
                })?;
            if symbol.unit != analysis.unit
                || !analysis.declarations.contains(&symbol.id)
                || !matches!(
                    symbol.kind,
                    SymbolKind::Constant | SymbolKind::Signal | SymbolKind::Variable
                )
                || !objects.insert(object.symbol)
            {
                return Err(ArtifactError::InvalidObject(object.symbol));
            }
            let type_id = symbol
                .type_id
                .ok_or(ArtifactError::InvalidObject(object.symbol))?;
            match &object.initialization {
                ObjectInitialization::Default(value) => {
                    if symbol.kind == SymbolKind::Constant {
                        return Err(ArtifactError::InvalidObject(object.symbol));
                    }
                    let native = native_type(self, type_id).map_err(|error| {
                        ArtifactError::InvalidInitialization {
                            symbol: object.symbol,
                            message: error.to_string(),
                        }
                    })?;
                    check_value(*value, native).map_err(|error| {
                        ArtifactError::InvalidInitialization {
                            symbol: object.symbol,
                            message: error.to_string(),
                        }
                    })?;
                }
                ObjectInitialization::Expression(plan) => {
                    plan.validate_in_context(self).map_err(|error| {
                        ArtifactError::InvalidInitialization {
                            symbol: object.symbol,
                            message: error.to_string(),
                        }
                    })?;
                    if plan
                        .nodes
                        .iter()
                        .find(|node| node.id == plan.result)
                        .map(|node| node.type_id)
                        != Some(type_id)
                    {
                        return Err(ArtifactError::InvalidObject(object.symbol));
                    }
                }
            }
        }
        let mut processes = BTreeSet::new();
        for process in &analysis.pending_processes {
            self.unit_data_span(process.span)?;
            let scope = self
                .scopes
                .iter()
                .find(|scope| scope.id == process.scope)
                .ok_or(ArtifactError::DanglingReference {
                    table: TableKind::Scope,
                    id: process.scope.0,
                })?;
            if scope.kind != ScopeKind::Lexical(analysis.unit) || !processes.insert(process.scope) {
                return Err(ArtifactError::InvalidProcess(process.scope));
            }
            if let Some(name) = &process.name {
                super::validate::name(name)?;
            }
        }
        Ok(())
    }
}
