mod render;

use std::{fmt, path::Path};

use serde::{Deserialize, Serialize};

use crate::source::ByteSpan;

pub use render::render;

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiagnosticStage {
    CommandLine,
    Analysis,
    Elaboration,
    Simulation,
    Artifact,
}

impl fmt::Display for DiagnosticStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CommandLine => "command line",
            Self::Analysis => "analysis",
            Self::Elaboration => "elaboration",
            Self::Simulation => "simulation",
            Self::Artifact => "artifact",
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Severity {
    Advice,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiagnosticCode {
    Syntax,
    TypeMismatch,
    CliUnimplemented,
    CliUsage,
    CliIo,
    ArtifactFormat,
    ArtifactLimit,
    ArtifactCompatibility,
    ArtifactValidation,
    ArtifactIo,
    SourceUnavailable,
    SourceTooLarge,
    PslUnsupported,
    DuplicateDeclaration,
    MissingUnit,
    WrongSecondaryParent,
    MissingLibrary,
    UnsupportedFeature,
    MissingName,
    Arithmetic,
    LiteralRange,
    AmbiguousOverload,
    MissingInitializer,
    NameMismatch,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Syntax => "OSV-SYNTAX",
            Self::TypeMismatch => "OSV-TYPE-MISMATCH",
            Self::CliUnimplemented => "OSV-CLI-UNIMPLEMENTED",
            Self::CliUsage => "OSV-CLI-USAGE",
            Self::CliIo => "OSV-CLI-IO",
            Self::ArtifactFormat => "OSV-ARTIFACT-FORMAT",
            Self::ArtifactLimit => "OSV-ARTIFACT-LIMIT",
            Self::ArtifactCompatibility => "OSV-ARTIFACT-COMPATIBILITY",
            Self::ArtifactValidation => "OSV-ARTIFACT-VALIDATION",
            Self::ArtifactIo => "OSV-ARTIFACT-IO",
            Self::SourceUnavailable => "OSV-SOURCE-UNAVAILABLE",
            Self::SourceTooLarge => "OSV-SOURCE-TOO-LARGE",
            Self::PslUnsupported => "OSV-PSL-UNSUPPORTED",
            Self::DuplicateDeclaration => "OSV-DUPLICATE-DECLARATION",
            Self::MissingUnit => "OSV-MISSING-UNIT",
            Self::WrongSecondaryParent => "OSV-WRONG-SECONDARY-PARENT",
            Self::MissingLibrary => "OSV-MISSING-LIBRARY",
            Self::UnsupportedFeature => "OSV-UNSUPPORTED-FEATURE",
            Self::MissingName => "OSV-MISSING-NAME",
            Self::Arithmetic => "OSV-ARITHMETIC",
            Self::LiteralRange => "OSV-LITERAL-RANGE",
            Self::AmbiguousOverload => "OSV-AMBIGUOUS-OVERLOAD",
            Self::MissingInitializer => "OSV-MISSING-INITIALIZER",
            Self::NameMismatch => "OSV-NAME-MISMATCH",
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticLabel {
    pub span: ByteSpan,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DiagnosticNote {
    Text(String),
    ExpectedActual { expected: String, actual: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ContextKind {
    Unit,
    Instance,
    Call,
}

impl fmt::Display for ContextKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unit => "unit",
            Self::Instance => "instance",
            Self::Call => "call",
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiagnosticContext {
    pub kind: ContextKind,
    pub name: String,
    pub span: Option<ByteSpan>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub stage: DiagnosticStage,
    pub severity: Severity,
    pub message: String,
    pub primary: Option<DiagnosticLabel>,
    pub related: Vec<DiagnosticLabel>,
    pub notes: Vec<DiagnosticNote>,
    pub context: Vec<DiagnosticContext>,
}

impl Diagnostic {
    pub fn new(
        code: DiagnosticCode,
        stage: DiagnosticStage,
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            stage,
            severity,
            message: message.into(),
            primary: None,
            related: Vec::new(),
            notes: Vec::new(),
            context: Vec::new(),
        }
    }

    pub fn with_primary(mut self, span: ByteSpan, message: impl Into<String>) -> Self {
        self.primary = Some(DiagnosticLabel {
            span,
            message: message.into(),
        });
        self
    }

    pub fn with_related(mut self, span: ByteSpan, message: impl Into<String>) -> Self {
        self.related.push(DiagnosticLabel {
            span,
            message: message.into(),
        });
        self
    }

    pub fn with_note(mut self, note: DiagnosticNote) -> Self {
        self.notes.push(note);
        self
    }

    pub fn with_context(mut self, context: DiagnosticContext) -> Self {
        self.context.push(context);
        self
    }

    pub fn exit_status(&self) -> u8 {
        match (self.severity, self.stage) {
            (Severity::Error, DiagnosticStage::CommandLine) => 2,
            (Severity::Error, _) => 1,
            _ => 0,
        }
    }
}

pub fn exit_status(diagnostics: &[Diagnostic]) -> u8 {
    diagnostics
        .iter()
        .map(Diagnostic::exit_status)
        .max()
        .unwrap_or(0)
}
