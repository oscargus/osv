use std::{
    io::{self, Write},
    ops::Range,
};

use vhdl_syntax::{
    parser,
    standard::VHDLStandard,
    syntax::{
        AstNode, DesignFileSyntax, PreorderWithTokens, SyntaxNode as ParserNode,
        SyntaxToken as ParserToken, WalkEvent,
    },
};

pub use vhdl_syntax::syntax::{NodeKind, TokenKind};
pub use vhdl_syntax::tokens::Keyword;

use crate::{
    diagnostics::{Diagnostic, DiagnosticCode, DiagnosticNote, DiagnosticStage, Severity},
    source::{ByteSpan, SourceFile, SourceId},
};

#[derive(Clone, Debug)]
pub struct ParsedSource {
    source: SourceId,
    length: u32,
    design: DesignFileSyntax,
}

impl ParsedSource {
    pub fn source_id(&self) -> SourceId {
        self.source
    }
    pub fn span(&self) -> ByteSpan {
        ByteSpan {
            source: self.source,
            start: 0,
            end: self.length,
        }
    }
    pub fn write_to(&self, output: &mut impl Write) -> io::Result<()> {
        self.design.raw().write_to(output)
    }
    pub fn root(&self) -> SyntaxNode {
        SyntaxNode {
            source: self.source,
            length: self.length,
            node: self.design.raw(),
        }
    }
    pub fn design_units(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.design.children().map(|node| SyntaxNode {
            source: self.source,
            length: self.length,
            node,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SyntaxNode {
    source: SourceId,
    length: u32,
    node: ParserNode,
}

impl SyntaxNode {
    pub fn kind(&self) -> NodeKind {
        self.node.kind()
    }
    pub fn span(&self) -> Result<ByteSpan, Box<Diagnostic>> {
        location(self.source, self.length, self.node.text_range())
    }
    pub fn children(&self) -> impl Iterator<Item = Self> + '_ {
        self.node.children().map(|node| Self {
            source: self.source,
            length: self.length,
            node,
        })
    }
    pub fn descendants(&self) -> impl Iterator<Item = Self> + '_ {
        PreorderWithTokens::new(self.node.clone()).filter_map(|event| match event {
            WalkEvent::Enter(node) => Some(Self {
                source: self.source,
                length: self.length,
                node,
            }),
            _ => None,
        })
    }
    pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.node.tokens().map(|token| SyntaxToken {
            source: self.source,
            length: self.length,
            token,
        })
    }
    pub fn all_tokens(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        PreorderWithTokens::new(self.node.clone()).filter_map(|event| match event {
            WalkEvent::Token(token) => Some(SyntaxToken {
                source: self.source,
                length: self.length,
                token,
            }),
            _ => None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SyntaxToken {
    source: SourceId,
    length: u32,
    token: ParserToken,
}

impl SyntaxToken {
    pub fn kind(&self) -> TokenKind {
        self.token.kind()
    }
    pub fn span(&self) -> Result<ByteSpan, Box<Diagnostic>> {
        location(self.source, self.length, self.token.text_range())
    }
    pub fn text(&self) -> String {
        self.token.text().to_string()
    }
}

fn location(
    source: SourceId,
    length: u32,
    range: Range<usize>,
) -> Result<ByteSpan, Box<Diagnostic>> {
    match (u32::try_from(range.start), u32::try_from(range.end)) {
        (Ok(start), Ok(end)) if start <= end && end <= length => {
            Ok(ByteSpan { source, start, end })
        }
        _ => Err(Box::new(
            Diagnostic::new(
                DiagnosticCode::Syntax,
                DiagnosticStage::Analysis,
                Severity::Error,
                "parser returned an invalid source span",
            )
            .with_note(DiagnosticNote::Text(format!(
                "byte offsets: {}..{}",
                range.start, range.end
            ))),
        )),
    }
}

fn psl_diagnostics(
    source: &SourceFile,
    length: u32,
    design: &DesignFileSyntax,
    bytes: &[u8],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for event in PreorderWithTokens::new(design.raw()) {
        let WalkEvent::Token(token) = event else {
            continue;
        };
        let mut offset = token.offset();
        for piece in token.leading_trivia().iter() {
            let end = offset + piece.byte_len();
            if piece.is_comment()
                && let Some(comment) = bytes
                    .get(offset..end)
                    .and_then(|comment| comment.strip_prefix(b"--"))
            {
                let comment = comment.trim_ascii_start();
                if comment
                    .get(..3)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"psl"))
                    && comment
                        .get(3)
                        .is_none_or(|next| next.is_ascii_whitespace() || *next == 0xa0)
                {
                    let diagnostic = Diagnostic::new(DiagnosticCode::PslUnsupported, DiagnosticStage::Analysis, Severity::Error, "embedded PSL is not supported by the pinned parser")
                        .with_note(DiagnosticNote::Text("coverage L067; lang.psl: parser support is required before PSL can execute".into()));
                    diagnostics.push(match location(source.id, length, offset..end) {
                        Ok(span) => {
                            diagnostic.with_primary(span, "PSL must not be silently ignored")
                        }
                        Err(error) => *error,
                    });
                }
            }
            offset = end;
        }
    }
    diagnostics
}

pub fn parse(source: &SourceFile) -> Result<ParsedSource, Vec<Diagnostic>> {
    let bytes = source.bytes.as_ref().ok_or_else(|| {
        vec![
            Diagnostic::new(
                DiagnosticCode::SourceUnavailable,
                DiagnosticStage::Analysis,
                Severity::Error,
                "VHDL source bytes are unavailable",
            )
            .with_primary(
                ByteSpan {
                    source: source.id,
                    start: 0,
                    end: 0,
                },
                "cannot parse this source",
            ),
        ]
    })?;
    let length = u32::try_from(bytes.len()).map_err(|_| {
        vec![Diagnostic::new(
            DiagnosticCode::SourceTooLarge,
            DiagnosticStage::Analysis,
            Severity::Error,
            "VHDL source exceeds the 32-bit byte-span limit",
        )]
    })?;
    let (design, errors) = parser::parse_with_standard(VHDLStandard::VHDL2008, bytes);
    let mut diagnostics = errors
        .iter()
        .map(|error| {
            let diagnostic = Diagnostic::new(
                DiagnosticCode::Syntax,
                DiagnosticStage::Analysis,
                Severity::Error,
                error.err().to_string(),
            );
            let range = error.span();
            match location(source.id, length, range.clone()) {
                Ok(span) => diagnostic.with_primary(span, "syntax error in VHDL 2008 source"),
                Err(error) => *error,
            }
        })
        .collect::<Vec<_>>();
    diagnostics.extend(psl_diagnostics(source, length, &design, bytes));
    if diagnostics.is_empty() {
        Ok(ParsedSource {
            source: source.id,
            length,
            design,
        })
    } else {
        Err(diagnostics)
    }
}
