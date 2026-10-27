#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/golden.rs"]
mod golden;

use fixture::{
    Fixture, FixtureKind, Metadata, Observation, TraceEvent, normalized_trace, round_trips,
};
use osv::{
    diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity, exit_status},
    source::ByteSpan,
};
use serde::{Deserialize, Serialize};
use vhdl_syntax::{parser, standard::VHDLStandard, syntax::AstNode};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ParserSummary {
    design_units: u32,
}

fn parser_probe(fixture: &Fixture<ParserSummary>) -> Observation<ParserSummary> {
    let mut diagnostics = Vec::new();
    let mut output = Vec::new();
    let mut design_units = 0;
    for source in fixture.sources.iter() {
        let bytes = source.bytes.as_ref().unwrap();
        let (design, errors) = parser::parse_with_standard(VHDLStandard::VHDL2008, bytes);
        design_units += u32::try_from(design.children().count()).unwrap();
        let mut reproduced = Vec::new();
        design.raw().write_to(&mut reproduced).unwrap();
        assert_eq!(&reproduced, bytes);
        if !errors.is_empty() {
            output.push(parser::error::display_errors(&errors));
        }
        for error in &errors {
            let range = error.span();
            let span = ByteSpan {
                source: source.id,
                start: u32::try_from(range.start).unwrap(),
                end: u32::try_from(range.end).unwrap(),
            };
            assert!(span.checked_range(bytes.len()).is_some());
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::Syntax,
                    DiagnosticStage::Analysis,
                    Severity::Error,
                    parser::error::display_errors(std::slice::from_ref(error)),
                )
                .with_primary(span, "parser probe rejected this source"),
            );
        }
    }
    Observation {
        kind: FixtureKind::ParserProbe,
        stage: DiagnosticStage::Analysis,
        exit_status: exit_status(&diagnostics),
        diagnostics,
        result: ParserSummary { design_units },
        output: output.join("\n"),
    }
}

#[test]
fn fixture_parser_probes_match_metadata() {
    for name in ["parser/empty-entity.ron", "parser/incomplete-entity.ron"] {
        let fixture = Fixture::load(name).unwrap();
        let actual = parser_probe(&fixture);
        fixture.check(&actual).unwrap();
    }
}

#[test]
fn fixture_parser_wrong_expected_output_fails() {
    let mut fixture = Fixture::load("parser/incomplete-entity.ron").unwrap();
    let actual = parser_probe(&fixture);
    fixture
        .metadata
        .expected
        .output
        .push_str(" deliberately wrong");
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("output mismatch")
    );
}

#[test]
fn fixture_parser_wrong_expected_stage_fails() {
    let mut fixture = Fixture::load("parser/incomplete-entity.ron").unwrap();
    let actual = parser_probe(&fixture);
    fixture.metadata.expected.stage = DiagnosticStage::Elaboration;
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("stage mismatch")
    );
}

#[test]
fn fixture_parser_missing_expected_diagnostic_fails() {
    let mut fixture = Fixture::load("parser/incomplete-entity.ron").unwrap();
    let mut actual = parser_probe(&fixture);
    let diagnostic = actual.diagnostics.pop().unwrap();
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("diagnostics mismatch")
    );
    actual.diagnostics.push(diagnostic);
    fixture.metadata.expected.diagnostics.clear();
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("diagnostics mismatch")
    );
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SelfTestDesign {
    top: String,
    architecture: String,
    signals: Vec<SelfTestSignal>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SelfTestSignal {
    name: String,
    initial: bool,
}

fn self_test_design() -> SelfTestDesign {
    SelfTestDesign {
        top: "synthetic.root".into(),
        architecture: "sample".into(),
        signals: vec![SelfTestSignal {
            name: "synthetic.root.flag".into(),
            initial: false,
        }],
    }
}

fn self_test_observation<T>(stage: DiagnosticStage, result: T) -> Observation<T> {
    Observation {
        kind: FixtureKind::HarnessSelfTest,
        stage,
        diagnostics: Vec::new(),
        exit_status: 0,
        result,
        output: String::new(),
    }
}

#[test]
fn fixture_self_test_artifact_round_trips_and_elaboration_results() {
    let original = self_test_design();
    for (name, stage) in [
        ("self-test/artifact.ron", DiagnosticStage::Artifact),
        ("self-test/elaboration.ron", DiagnosticStage::Elaboration),
    ] {
        let fixture = Fixture::load(name).unwrap();
        for result in round_trips(&original).unwrap() {
            let actual = self_test_observation(stage, result);
            fixture.check(&actual).unwrap();
        }
        let mut changed = original.clone();
        changed.signals[0].initial = true;
        let actual = self_test_observation(stage, changed);
        assert!(
            fixture
                .check(&actual)
                .unwrap_err()
                .contains("result mismatch")
        );
    }
}

#[test]
fn fixture_parser_sources_and_diagnostics_survive_codec_conversions() {
    let fixture = Fixture::load("parser/incomplete-entity.ron").unwrap();
    let actual = parser_probe(&fixture);
    let original = (
        fixture.sources.iter().cloned().collect::<Vec<_>>(),
        actual.diagnostics.clone(),
    );
    let output = golden::rendered(&actual.diagnostics[0], &fixture.sources);
    let snapshot = ron::to_string(&output).unwrap();
    assert_eq!(golden::expected_output(&snapshot), output);
    for (sources, diagnostics) in round_trips(&original).unwrap() {
        let mut restored = osv::source::SourceMap::default();
        for source in sources {
            restored.insert(source).unwrap();
        }
        assert_eq!(golden::rendered(&diagnostics[0], &restored), output);
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum SelfTestPhase {
    Update,
    Observe,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum SelfTestEvent {
    Signal { name: String, value: bool },
    Report(String),
}

fn self_test_trace() -> Vec<TraceEvent<SelfTestPhase, SelfTestEvent>> {
    normalized_trace(
        1_000,
        [
            (
                0,
                0,
                SelfTestPhase::Update,
                SelfTestEvent::Signal {
                    name: "synthetic.root.flag".into(),
                    value: false,
                },
            ),
            (
                0,
                1,
                SelfTestPhase::Update,
                SelfTestEvent::Signal {
                    name: "synthetic.root.flag".into(),
                    value: true,
                },
            ),
            (
                1,
                0,
                SelfTestPhase::Observe,
                SelfTestEvent::Report("harness sample".into()),
            ),
        ],
    )
    .unwrap()
}

#[test]
fn fixture_self_test_trace_is_normalized_and_codec_independent() {
    let fixture = Fixture::load("self-test/trace.ron").unwrap();
    let original = self_test_trace();
    for trace in round_trips(&original).unwrap() {
        let actual = self_test_observation(DiagnosticStage::Simulation, trace);
        fixture.check(&actual).unwrap();
    }
}

#[test]
fn fixture_self_test_trace_preserves_values_times_deltas_phases_and_order() {
    let fixture = Fixture::load("self-test/trace.ron").unwrap();
    let original = self_test_trace();
    for index in 0..5 {
        let mut changed = original.clone();
        match index {
            0 => {
                if let SelfTestEvent::Signal { value, .. } = &mut changed[0].event {
                    *value = true;
                }
            }
            1 => changed[2].time_fs += 1,
            2 => changed[1].delta += 1,
            3 => changed[0].phase = SelfTestPhase::Observe,
            4 => changed.swap(0, 1),
            _ => unreachable!(),
        }
        let actual = self_test_observation(DiagnosticStage::Simulation, changed);
        assert!(
            fixture
                .check(&actual)
                .unwrap_err()
                .contains("result mismatch")
        );
    }
}

#[test]
fn fixture_trace_rejects_resolution_overflow_and_time_or_delta_regressions() {
    assert!(
        normalized_trace::<(), ()>(0, [])
            .unwrap_err()
            .contains("resolution")
    );
    assert!(
        normalized_trace(2, [(u64::MAX, 0, (), ())])
            .unwrap_err()
            .contains("overflow")
    );
    for positions in [[(1, 0), (0, 0)], [(0, 1), (0, 0)]] {
        let events = positions.map(|(ticks, delta)| (ticks, delta, (), ()));
        assert!(
            normalized_trace(1, events)
                .unwrap_err()
                .contains("regressed")
        );
    }
    assert!(normalized_trace::<(), ()>(1, []).unwrap().is_empty());
    let boundary = normalized_trace(1, [(u64::MAX, u64::MAX, (), ())]).unwrap();
    assert_eq!(boundary[0].time_fs, u64::MAX);
    assert_eq!(boundary[0].delta, u64::MAX);
}

#[test]
fn fixture_parser_extra_diagnostic_and_wrong_code_or_status_fail() {
    let fixture = Fixture::load("parser/incomplete-entity.ron").unwrap();
    let mut actual = parser_probe(&fixture);
    actual.exit_status = 0;
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("exit status mismatch")
    );
    actual.exit_status = 1;
    actual.diagnostics[0].code = DiagnosticCode::TypeMismatch;
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("diagnostics mismatch")
    );
    actual.diagnostics[0].code = DiagnosticCode::Syntax;
    actual.diagnostics.push(actual.diagnostics[0].clone());
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("diagnostics mismatch")
    );
}

#[test]
fn fixture_parser_and_self_test_kinds_are_distinct() {
    let fixture = Fixture::load("parser/empty-entity.ron").unwrap();
    let mut actual = parser_probe(&fixture);
    actual.kind = FixtureKind::HarnessSelfTest;
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("fixture kind mismatch")
    );
}

#[test]
fn fixture_diagnostic_stage_and_severity_are_semantic_expectations() {
    let fixture = Fixture::load("parser/incomplete-entity.ron").unwrap();
    let mut actual = parser_probe(&fixture);
    actual.diagnostics[0].stage = DiagnosticStage::Simulation;
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("diagnostics mismatch")
    );
    actual.diagnostics[0].stage = DiagnosticStage::Analysis;
    actual.diagnostics[0].severity = Severity::Warning;
    assert!(
        fixture
            .check(&actual)
            .unwrap_err()
            .contains("diagnostics mismatch")
    );
}

#[test]
fn fixture_trace_retains_duplicate_events_at_the_same_position() {
    let events = [
        SelfTestPhase::Update,
        SelfTestPhase::Observe,
        SelfTestPhase::Update,
    ]
    .map(|phase| (0, 0, phase, false));
    let trace = normalized_trace(1_000, events).unwrap();
    assert_eq!(trace.len(), 3);
    assert_eq!(
        trace.iter().map(|event| event.phase).collect::<Vec<_>>(),
        [
            SelfTestPhase::Update,
            SelfTestPhase::Observe,
            SelfTestPhase::Update,
        ]
    );
    assert!(trace.iter().all(|event| !event.event));
}

#[test]
fn fixture_metadata_is_strict_and_accepts_lf_or_crlf_checkouts() {
    let text = include_str!("fixtures/parser/incomplete-entity.ron").replace("\r\n", "\n");
    let unix = Metadata::<ParserSummary>::from_ron(&text).unwrap();
    let windows = Metadata::<ParserSummary>::from_ron(&text.replace('\n', "\r\n")).unwrap();
    assert_eq!(unix.expected.stage, windows.expected.stage);
    assert_eq!(unix.expected.diagnostics, windows.expected.diagnostics);
    assert_eq!(unix.expected.result, windows.expected.result);
    assert_eq!(unix.expected.output, windows.expected.output);
    for invalid in [
        text.replace("stage: Analysis", "stage: NotAStage"),
        text.replace("exit_status: 1", "exit_sttaus: 1"),
        text.replace("code: Syntax", "code: NotADiagnosticCode"),
        text.replace("design_units: 1", "design_units: 1, ignored: true"),
    ] {
        assert!(Metadata::<ParserSummary>::from_ron(&invalid).is_err());
    }
    let trace = include_str!("fixtures/self-test/trace.ron")
        .replace("time_fs: 0", "time_fs: 0, ignored: true");
    assert!(Metadata::<Vec<TraceEvent<SelfTestPhase, SelfTestEvent>>>::from_ron(&trace).is_err());
}

#[test]
fn fixture_missing_files_and_nonportable_paths_are_errors() {
    for name in [
        "../project.md",
        "/tmp/outside.ron",
        r"parser\empty-entity.ron",
        "C:/outside.ron",
    ] {
        let error = Fixture::<ParserSummary>::load(name).err().unwrap();
        assert!(error.contains("portable and relative"), "{error}");
    }
    let error = Fixture::<ParserSummary>::load("missing.ron").err().unwrap();
    assert!(error.contains("cannot read metadata"));
}
