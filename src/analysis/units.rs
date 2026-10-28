use crate::{
    artifacts::{
        ArtifactEnvelope, ArtifactError, Identifier, ScopeId, ScopeKind, ScopeRecord, SymbolId,
        UnitId, UnitIdentity, UnitKind, UnitRecord,
    },
    diagnostics::{Diagnostic, DiagnosticCode, DiagnosticNote, DiagnosticStage, Severity},
    source::ByteSpan,
    syntax::{NodeKind, ParsedSource, SyntaxNode, SyntaxToken, TokenKind},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationKind {
    Type,
    Subtype,
    Constant,
    Signal,
    Variable,
    Subprogram,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclarationName {
    pub id: SymbolId,
    pub unit: UnitId,
    pub scope: ScopeId,
    pub name: Identifier,
    pub kind: DeclarationKind,
    pub span: Option<ByteSpan>,
}

#[derive(Clone, Debug)]
pub struct UnitRegistry {
    library: Identifier,
    units: Vec<UnitRecord>,
    scopes: Vec<ScopeRecord>,
    declarations: Vec<DeclarationName>,
}

fn error(code: DiagnosticCode, span: ByteSpan, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(code, DiagnosticStage::Analysis, Severity::Error, message)
        .with_primary(span, "declaration registration failed")
}

pub(crate) fn identifier(token: &SyntaxToken) -> Option<Identifier> {
    let text = token.text();
    if let Some(inner) = text
        .strip_prefix('\\')
        .and_then(|text| text.strip_suffix('\\'))
    {
        Some(Identifier::Extended(inner.replace("\\\\", "\\")))
    } else if token.kind() == TokenKind::Identifier {
        Some(Identifier::Basic(text).normalized())
    } else {
        None
    }
}

fn named_token(token: SyntaxToken) -> Option<Result<(Identifier, ByteSpan), Box<Diagnostic>>> {
    identifier(&token).map(|name| token.span().map(|span| (name, span)))
}

fn identifiers(
    node: &SyntaxNode,
) -> impl Iterator<Item = Result<(Identifier, ByteSpan), Box<Diagnostic>>> + '_ {
    node.all_tokens().filter_map(named_token)
}

fn next_id(ids: impl Iterator<Item = u32>, span: ByteSpan) -> Result<u32, Box<Diagnostic>> {
    match ids.max() {
        None => Ok(0),
        Some(id) => id.checked_add(1).ok_or_else(|| {
            Box::new(error(
                DiagnosticCode::UnsupportedFeature,
                span,
                "declaration ID space exhausted",
            ))
        }),
    }
}

impl UnitRegistry {
    pub fn new(library: Identifier) -> Self {
        let library = library.normalized();
        let mut scopes = vec![ScopeRecord {
            id: ScopeId(0),
            parent: None,
            kind: ScopeKind::Library(library.clone()),
        }];
        let standard = Identifier::Basic("std".into());
        if library != standard {
            scopes.push(ScopeRecord {
                id: ScopeId(1),
                parent: None,
                kind: ScopeKind::Library(standard),
            });
        }
        Self {
            library,
            units: Vec::new(),
            scopes,
            declarations: Vec::new(),
        }
    }

    pub fn from_artifact(artifact: &ArtifactEnvelope) -> Result<Self, ArtifactError> {
        artifact.validate(artifact.stage())?;
        let declarations = artifact
            .symbols
            .iter()
            .map(|symbol| DeclarationName {
                id: symbol.id,
                unit: symbol.unit,
                scope: symbol.scope,
                name: symbol.name.clone(),
                kind: match symbol.kind {
                    crate::artifacts::SymbolKind::Type => DeclarationKind::Type,
                    crate::artifacts::SymbolKind::Constant => DeclarationKind::Constant,
                    crate::artifacts::SymbolKind::Signal => DeclarationKind::Signal,
                    crate::artifacts::SymbolKind::Variable => DeclarationKind::Variable,
                    crate::artifacts::SymbolKind::Subprogram => DeclarationKind::Subprogram,
                    crate::artifacts::SymbolKind::EnumerationLiteral
                    | crate::artifacts::SymbolKind::PhysicalUnit => DeclarationKind::Constant,
                    crate::artifacts::SymbolKind::Attribute => DeclarationKind::Type,
                },
                span: symbol.span,
            })
            .collect();
        Ok(Self {
            library: artifact.identity.library.normalized(),
            units: artifact.units.clone(),
            scopes: artifact.scopes.clone(),
            declarations,
        })
    }

    pub fn units(&self) -> &[UnitRecord] {
        &self.units
    }
    pub fn scopes(&self) -> &[ScopeRecord] {
        &self.scopes
    }
    pub fn declarations(&self) -> &[DeclarationName] {
        &self.declarations
    }

    pub fn library_scope(&self, library: &Identifier) -> Option<ScopeId> {
        let library = self.library_alias(library);
        self.scopes.iter().find_map(|scope| match &scope.kind {
            ScopeKind::Library(name) if name.normalized() == library => Some(scope.id),
            _ => None,
        })
    }

    fn library_alias(&self, library: &Identifier) -> Identifier {
        if library.normalized() == Identifier::Basic("work".into()) {
            self.library.clone()
        } else {
            library.normalized()
        }
    }

    pub fn lookup_unit(
        &self,
        library: &Identifier,
        primary: &Identifier,
        kind: &UnitKind,
    ) -> Option<&UnitRecord> {
        let identity = UnitIdentity {
            library: self.library_alias(library),
            primary: primary.normalized(),
            kind: kind.clone(),
        }
        .normalized();
        self.units
            .iter()
            .find(|unit| unit.identity.normalized() == identity)
    }

    pub fn lookup(
        &self,
        mut scope: ScopeId,
        name: &Identifier,
    ) -> Result<Vec<&DeclarationName>, ArtifactError> {
        loop {
            let record = self.scopes.iter().find(|record| record.id == scope).ok_or(
                ArtifactError::DanglingReference {
                    table: crate::artifacts::TableKind::Scope,
                    id: scope.0,
                },
            )?;
            let found = self
                .declarations
                .iter()
                .filter(|declaration| {
                    declaration.scope == scope && declaration.name.normalized() == name.normalized()
                })
                .collect::<Vec<_>>();
            if !found.is_empty() {
                return Ok(found);
            }
            let Some(parent) = record.parent else {
                return Ok(Vec::new());
            };
            scope = parent;
        }
    }

    pub fn register(&mut self, parsed: &ParsedSource) -> Result<Vec<UnitId>, Vec<Diagnostic>> {
        let mut next = self.clone();
        let mut registered = Vec::new();
        for wrapper in parsed.design_units() {
            match next.register_unit(&wrapper) {
                Ok(id) => registered.push(id),
                Err(error) => return Err(vec![*error]),
            }
        }
        *self = next;
        Ok(registered)
    }

    fn new_scope(
        &mut self,
        parent: ScopeId,
        kind: ScopeKind,
        span: ByteSpan,
    ) -> Result<ScopeId, Box<Diagnostic>> {
        let id = ScopeId(next_id(self.scopes.iter().map(|scope| scope.id.0), span)?);
        self.scopes.push(ScopeRecord {
            id,
            parent: Some(parent),
            kind,
        });
        Ok(id)
    }

    fn register_unit(&mut self, wrapper: &SyntaxNode) -> Result<UnitId, Box<Diagnostic>> {
        let span = wrapper.span()?;
        let node = wrapper
            .descendants()
            .find(|node| {
                matches!(
                    node.kind(),
                    NodeKind::EntityDeclaration
                        | NodeKind::ArchitectureBody
                        | NodeKind::PrimaryUnitPackageDeclaration
                        | NodeKind::SecondaryUnitPackageBody
                        | NodeKind::ConfigurationDeclaration
                        | NodeKind::ContextDeclaration
                )
            })
            .ok_or_else(|| {
                Box::new(error(
                    DiagnosticCode::UnsupportedFeature,
                    span,
                    "unsupported design-unit syntax; coverage L004",
                ))
            })?;
        self.context(wrapper, &node)?;
        let names = identifiers(&node).take(2).collect::<Result<Vec<_>, _>>()?;
        let (name, name_span) = names.first().cloned().ok_or_else(|| {
            Box::new(error(
                DiagnosticCode::Syntax,
                span,
                "design unit has no identifier",
            ))
        })?;
        let (primary, kind, expected_parent) = match node.kind() {
            NodeKind::EntityDeclaration => (name, UnitKind::Entity, None),
            NodeKind::PrimaryUnitPackageDeclaration => (name, UnitKind::Package, None),
            NodeKind::ConfigurationDeclaration => (name, UnitKind::Configuration, None),
            NodeKind::ContextDeclaration => (name, UnitKind::Context, None),
            NodeKind::SecondaryUnitPackageBody => {
                (name, UnitKind::PackageBody, Some(UnitKind::Package))
            }
            NodeKind::ArchitectureBody => {
                let parent = names
                    .get(1)
                    .ok_or_else(|| {
                        Box::new(error(
                            DiagnosticCode::Syntax,
                            span,
                            "architecture has no entity association",
                        ))
                    })?
                    .0
                    .clone();
                (
                    parent,
                    UnitKind::Architecture { name },
                    Some(UnitKind::Entity),
                )
            }
            _ => {
                return Err(Box::new(error(
                    DiagnosticCode::UnsupportedFeature,
                    span,
                    "unsupported design unit; coverage L004",
                )));
            }
        };
        let identity = UnitIdentity {
            library: self.library.clone(),
            primary,
            kind,
        }
        .normalized();
        let secondary = expected_parent.is_some();
        if let Some(previous) = self.units.iter().find(|previous| {
            previous.identity == identity
                || (!secondary
                    && previous.parent.is_none()
                    && previous.identity.library == identity.library
                    && previous.identity.primary == identity.primary)
        }) {
            let mut diagnostic = error(
                DiagnosticCode::DuplicateDeclaration,
                name_span,
                "duplicate design-unit declaration",
            );
            if let Some(span) = previous.span {
                diagnostic = diagnostic.with_related(span, "previous declaration is here");
            }
            return Err(Box::new(diagnostic));
        }
        let parent = if let Some(kind) = expected_parent {
            if let Some(parent) = self.lookup_unit(&self.library, &identity.primary, &kind) {
                Some(parent.id)
            } else {
                let wrong = self.units.iter().find(|unit| {
                    unit.identity.primary == identity.primary
                        && unit.identity.library == self.library
                        && unit.parent.is_none()
                });
                let code = if wrong.is_some() {
                    DiagnosticCode::WrongSecondaryParent
                } else {
                    DiagnosticCode::MissingUnit
                };
                let mut diagnostic = error(code, name_span, "secondary unit requires an earlier matching primary declaration")
                    .with_note(DiagnosticNote::Text("analyze the primary before its architecture or package body; coverage L004/L005".into()));
                if let Some(span) = wrong.and_then(|unit| unit.span) {
                    diagnostic = diagnostic.with_related(span, "this primary has the wrong kind");
                }
                return Err(Box::new(diagnostic));
            }
        } else {
            None
        };
        let id = UnitId(next_id(self.units.iter().map(|unit| unit.id.0), span)?);
        let parent_scope = if let Some(parent) = parent {
            self.scopes
                .iter()
                .find_map(|scope| (scope.kind == ScopeKind::Unit(parent)).then_some(scope.id))
        } else {
            self.library_scope(&self.library)
        }
        .ok_or_else(|| {
            Box::new(error(
                DiagnosticCode::MissingLibrary,
                span,
                "unit scope parent is unavailable",
            ))
        })?;
        let scope = self.new_scope(parent_scope, ScopeKind::Unit(id), span)?;
        self.units.push(UnitRecord {
            id,
            identity,
            parent,
            span: Some(name_span),
        });
        self.collect_declarations(&node, id, scope)?;
        Ok(id)
    }

    fn context(&self, wrapper: &SyntaxNode, unit: &SyntaxNode) -> Result<(), Box<Diagnostic>> {
        let unit_start = unit.span()?.start;
        for node in wrapper.descendants() {
            if node.span()?.start >= unit_start {
                continue;
            }
            match node.kind() {
                NodeKind::LibraryClause => {
                    for identifier in identifiers(&node) {
                        let (library, span) = identifier?;
                        if self.library_scope(&library).is_none() {
                            return Err(Box::new(error(
                                DiagnosticCode::MissingLibrary,
                                span,
                                "library is not available to the declaration registry",
                            )));
                        }
                    }
                }
                NodeKind::UseClause => {
                    return Err(Box::new(error(
                        DiagnosticCode::UnsupportedFeature,
                        node.span()?,
                        "use-clause visibility is not implemented in the initial name pass; coverage L006",
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn declare(
        &mut self,
        unit: UnitId,
        scope: ScopeId,
        name: Identifier,
        kind: DeclarationKind,
        span: ByteSpan,
    ) -> Result<(), Box<Diagnostic>> {
        if let Some(previous) = self.declarations.iter().find(|previous| {
            previous.scope == scope
                && previous.name.normalized() == name.normalized()
                && !(previous.kind == DeclarationKind::Subprogram
                    && kind == DeclarationKind::Subprogram)
        }) {
            let mut diagnostic = error(
                DiagnosticCode::DuplicateDeclaration,
                span,
                "duplicate lexical declaration",
            );
            if let Some(span) = previous.span {
                diagnostic = diagnostic.with_related(span, "previous declaration is here");
            }
            return Err(Box::new(diagnostic));
        }
        let id = SymbolId(next_id(
            self.declarations.iter().map(|declaration| declaration.id.0),
            span,
        )?);
        self.declarations.push(DeclarationName {
            id,
            unit,
            scope,
            name,
            kind,
            span: Some(span),
        });
        Ok(())
    }

    fn collect_declarations(
        &mut self,
        root: &SyntaxNode,
        unit: UnitId,
        scope: ScopeId,
    ) -> Result<(), Box<Diagnostic>> {
        let mut pending = root
            .children()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|node| (node, scope))
            .collect::<Vec<_>>();
        while let Some((node, mut scope)) = pending.pop() {
            let kind = match node.kind() {
                NodeKind::FullTypeDeclaration => Some(DeclarationKind::Type),
                NodeKind::SubtypeDeclaration => Some(DeclarationKind::Subtype),
                NodeKind::ConstantDeclaration => Some(DeclarationKind::Constant),
                NodeKind::SignalDeclaration => Some(DeclarationKind::Signal),
                NodeKind::VariableDeclaration => Some(DeclarationKind::Variable),
                NodeKind::SubprogramDeclaration | NodeKind::SubprogramBody => {
                    Some(DeclarationKind::Subprogram)
                }
                _ => None,
            };
            if let Some(kind) = kind {
                let names = if matches!(
                    kind,
                    DeclarationKind::Constant | DeclarationKind::Signal | DeclarationKind::Variable
                ) {
                    node.all_tokens()
                        .take_while(|token| token.text() != ":")
                        .filter_map(named_token)
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    identifiers(&node).take(1).collect::<Result<Vec<_>, _>>()?
                };
                for (name, span) in names {
                    self.declare(unit, scope, name, kind, span)?;
                }
            }
            if matches!(
                node.kind(),
                NodeKind::ProcessStatement | NodeKind::BlockStatement | NodeKind::SubprogramBody
            ) {
                scope = self.new_scope(scope, ScopeKind::Lexical(unit), node.span()?)?;
            }
            for child in node.children().collect::<Vec<_>>().into_iter().rev() {
                pending.push((child, scope));
            }
        }
        Ok(())
    }
}
