#[allow(dead_code)]
#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/golden.rs"]
mod golden;
#[path = "support/artifact.rs"]
mod support;

use std::collections::BTreeMap;

use osv::{
    analysis::{ExpressionOptions, analyze_expression, constant::FoldResult},
    artifacts::*,
    diagnostics::{DiagnosticCode, DiagnosticStage, exit_status},
    intrinsics::{NativeBinding, NativeType, Operator, PreludeIds, install_standard},
    source::{SourceFile, SourceId},
    syntax::{self, NodeKind, SyntaxNode},
    values::*,
};

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
enum Outcome {
    Value(PrimitiveValue),
    Deferred,
    Failed,
}

fn prepare(source: &SourceFile) -> (ArtifactEnvelope, PreludeIds, SyntaxNode) {
    let parsed = syntax::parse(source).unwrap();
    let expression = parsed
        .root()
        .descendants()
        .find(|node| {
            matches!(
                node.kind(),
                NodeKind::BinaryExpression
                    | NodeKind::UnaryExpression
                    | NodeKind::LiteralExpression
                    | NodeKind::PhysicalLiteralExpression
                    | NodeKind::NameExpression
                    | NodeKind::ParenthesizedExpression
                    | NodeKind::ParenthesizedExpressionOrAggregate
            )
        })
        .unwrap();
    let mut artifact = support::sample();
    artifact.sources = vec![SourceRecord::embedded(source.clone()).unwrap()];
    let prelude = install_standard(&mut artifact).unwrap();
    (artifact, prelude, expression)
}

fn run_fixture(
    name: &str,
    native: NativeType,
) -> (
    ArtifactEnvelope,
    Option<osv::analysis::AnalyzedExpression>,
    Vec<osv::diagnostics::Diagnostic>,
) {
    let fixture = fixture::Fixture::<Outcome>::load(name).unwrap();
    let source = fixture.sources.iter().next().unwrap();
    let (artifact, prelude, expression) = prepare(source);
    let expected = prelude.types[&native];
    let result = analyze_expression(
        &expression,
        &artifact,
        ScopeId(1),
        Some(expected),
        &BTreeMap::new(),
        ExpressionOptions::default(),
    );
    let (result, diagnostics) = match result {
        Ok(result) => (Some(result), Vec::new()),
        Err(diagnostics) => (None, diagnostics),
    };
    assert!(
        result.is_some() || fixture.metadata.expected.exit_status != 0,
        "unexpected expression failure: {diagnostics:#?}"
    );
    let outcome = match result.as_ref().map(|result| &result.constant) {
        Some(FoldResult::Value(value)) => Outcome::Value(*value),
        Some(FoldResult::NotYetStatic) => Outcome::Deferred,
        None => Outcome::Failed,
    };
    fixture
        .check(&fixture::Observation {
            kind: fixture::FixtureKind::ExpressionAnalysis,
            stage: DiagnosticStage::Analysis,
            exit_status: exit_status(&diagnostics),
            diagnostics: diagnostics.clone(),
            result: outcome,
            output: String::new(),
        })
        .unwrap();
    if let Some(result) = &result {
        let unfurled = analyze_expression(
            &expression,
            &artifact,
            ScopeId(1),
            Some(expected),
            &BTreeMap::new(),
            ExpressionOptions {
                fold_constants: false,
            },
        )
        .unwrap();
        assert_eq!(unfurled.constant, result.constant);
        assert_eq!(
            unfurled.plan.evaluate(&artifact, &BTreeMap::new()).unwrap(),
            result.constant
        );
        for restored in fixture::round_trips(&result.plan).unwrap() {
            assert_eq!(
                restored.evaluate(&artifact, &BTreeMap::new()).unwrap(),
                result.constant
            );
        }
    }
    (artifact, result, diagnostics)
}

#[test]
fn syntax_expressions_type_real_vhdl_scalar_fixtures() {
    for (name, native) in [
        ("expressions/integer.ron", NativeType::Integer),
        ("expressions/boolean.ron", NativeType::Boolean),
        ("expressions/time.ron", NativeType::Time),
        ("expressions/deferred.ron", NativeType::Time),
        ("expressions/type-error.ron", NativeType::Integer),
        ("expressions/missing.ron", NativeType::Integer),
        ("expressions/overflow.ron", NativeType::Integer),
    ] {
        run_fixture(name, native);
    }
}

#[test]
fn syntax_expression_type_errors_label_both_operands_and_render_shared_diagnostics() {
    let (artifact, _, diagnostics) = run_fixture("expressions/type-error.ron", NativeType::Integer);
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.code, DiagnosticCode::TypeMismatch);
    assert_eq!(diagnostic.related.len(), 2);
    assert!(
        diagnostic
            .related
            .iter()
            .any(|label| label.message.contains("integer"))
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|label| label.message.contains("boolean"))
    );
    let mut sources = osv::source::SourceMap::default();
    for source in artifact.sources {
        sources.insert(source.file).unwrap();
    }
    let output = golden::rendered(diagnostic, &sources);
    assert_eq!(
        output,
        golden::expected_output(include_str!(
            "snapshots/diagnostics/expression-type-mismatch.ron"
        ))
    );
    assert!(output.contains("expected type:"));
    assert!(output.contains("actual type:"));
}

#[test]
fn syntax_expression_integer_endpoints_division_and_expected_type_ranges_are_checked() {
    for (text, native, expected) in [
        (
            "-9223372036854775808",
            NativeType::Integer,
            Some(PrimitiveValue::Integer(IntegerValue(i64::MIN))),
        ),
        (
            "9223372036854775807",
            NativeType::Integer,
            Some(PrimitiveValue::Integer(IntegerValue(i64::MAX))),
        ),
        ("1 / 0", NativeType::Integer, None),
        ("-1", NativeType::Natural, None),
        ("42", NativeType::Boolean, None),
    ] {
        let bytes = format!("package p is constant result : integer := {text}; end;");
        let source = SourceFile::new(
            SourceId(4),
            std::path::Path::new("boundary.vhd"),
            bytes.into_bytes(),
        );
        let (artifact, prelude, expression) = prepare(&source);
        let result = analyze_expression(
            &expression,
            &artifact,
            ScopeId(1),
            Some(prelude.types[&native]),
            &BTreeMap::new(),
            ExpressionOptions::default(),
        );
        match expected {
            Some(expected) => assert_eq!(result.unwrap().constant, FoldResult::Value(expected)),
            None => assert!(result.is_err(), "{text}"),
        }
    }
}

#[test]
fn syntax_expression_unsupported_real_aggregate_and_user_operator_are_explicit() {
    for text in ["1.5", "(1, 2)", "+2", "2 ns * 3", "false < true"] {
        let bytes = format!("package p is constant result : integer := {text}; end;");
        let source = SourceFile::new(
            SourceId(4),
            std::path::Path::new("unsupported.vhd"),
            bytes.into_bytes(),
        );
        let (artifact, prelude, expression) = prepare(&source);
        let errors = analyze_expression(
            &expression,
            &artifact,
            ScopeId(1),
            Some(prelude.types[&NativeType::Integer]),
            &BTreeMap::new(),
            ExpressionOptions::default(),
        )
        .err()
        .unwrap();
        assert_eq!(errors[0].code, DiagnosticCode::UnsupportedFeature);
    }
    let source = SourceFile::new(
        SourceId(4),
        std::path::Path::new("operator.vhd"),
        b"package p is constant result : integer := 1 + 2; end;",
    );
    let (mut artifact, prelude, expression) = prepare(&source);
    let id = artifact
        .symbols
        .iter()
        .map(|symbol| symbol.id.0)
        .max()
        .unwrap()
        + 1;
    artifact.symbols.push(SymbolRecord {
        id: SymbolId(id),
        unit: UnitId(7),
        scope: ScopeId(1),
        name: Identifier::Operator(Operator::Add),
        kind: SymbolKind::Subprogram,
        type_id: Some(prelude.types[&NativeType::Integer]),
        span: None,
        native: None,
    });
    let errors = analyze_expression(
        &expression,
        &artifact,
        ScopeId(1),
        None,
        &BTreeMap::new(),
        ExpressionOptions::default(),
    )
    .err()
    .unwrap();
    assert_eq!(errors[0].code, DiagnosticCode::UnsupportedFeature);
    assert!(
        artifact.symbols.last().unwrap().native != Some(NativeBinding::Type(NativeType::Integer))
    );
}
