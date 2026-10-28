use std::{
    fmt::Debug,
    fs,
    path::{Component, Path, PathBuf},
};

use osv::{
    diagnostics::{Diagnostic, DiagnosticCode, DiagnosticStage, Severity},
    source::SourceMap,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum FixtureKind {
    ParserProbe,
    HarnessSelfTest,
    ExpressionAnalysis,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata<T> {
    pub kind: FixtureKind,
    pub sources: Vec<String>,
    pub expected: Expected<T>,
}

impl<T: DeserializeOwned> Metadata<T> {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(&text.replace("\r\n", "\n")).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expected<T> {
    pub stage: DiagnosticStage,
    pub diagnostics: Vec<ExpectedDiagnostic>,
    pub exit_status: u8,
    pub result: T,
    pub output: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDiagnostic {
    pub stage: DiagnosticStage,
    pub code: DiagnosticCode,
    pub severity: Severity,
}

pub struct Observation<T> {
    pub kind: FixtureKind,
    pub stage: DiagnosticStage,
    pub diagnostics: Vec<Diagnostic>,
    pub exit_status: u8,
    pub result: T,
    pub output: String,
}

pub struct Fixture<T> {
    pub metadata: Metadata<T>,
    pub sources: SourceMap,
    name: String,
}

fn local_path(root: &Path, name: &str) -> Result<PathBuf, String> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains(['\\', ':'])
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "fixture path must be portable and relative: {name:?}"
        ));
    }
    Ok(root.join(path))
}

fn equal<T: Debug + PartialEq>(
    name: &str,
    field: &str,
    expected: &T,
    actual: &T,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "{name}: {field} mismatch\nexpected: {expected:#?}\nactual: {actual:#?}"
        ))
    }
}

impl<T: DeserializeOwned> Fixture<T> {
    pub fn load(name: &str) -> Result<Self, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let metadata_path = local_path(&root, name)?;
        let metadata = fs::read_to_string(&metadata_path)
            .map_err(|error| format!("{name}: cannot read metadata: {error}"))?;
        let metadata: Metadata<T> = Metadata::from_ron(&metadata)
            .map_err(|error| format!("{name}: invalid metadata: {error}"))?;
        if metadata.kind == FixtureKind::ParserProbe && metadata.sources.is_empty() {
            return Err(format!("{name}: parser probe has no sources"));
        }
        let mut sources = SourceMap::default();
        for source in &metadata.sources {
            let path = local_path(&root, source)?;
            let bytes = fs::read(&path)
                .map_err(|error| format!("{name}: cannot read {source}: {error}"))?;
            sources
                .add(&Path::new("fixtures").join(source), bytes)
                .map_err(|error| format!("{name}: {source}: {error}"))?;
        }
        Ok(Self {
            metadata,
            sources,
            name: name.into(),
        })
    }
}

impl<T: Debug + PartialEq> Fixture<T> {
    pub fn check(&self, actual: &Observation<T>) -> Result<(), String> {
        let expected = &self.metadata.expected;
        equal(
            &self.name,
            "fixture kind",
            &self.metadata.kind,
            &actual.kind,
        )?;
        equal(&self.name, "stage", &expected.stage, &actual.stage)?;
        equal(
            &self.name,
            "exit status",
            &expected.exit_status,
            &actual.exit_status,
        )?;
        let diagnostics = actual
            .diagnostics
            .iter()
            .map(|diagnostic| ExpectedDiagnostic {
                stage: diagnostic.stage,
                code: diagnostic.code,
                severity: diagnostic.severity,
            })
            .collect::<Vec<_>>();
        equal(
            &self.name,
            "diagnostics",
            &expected.diagnostics,
            &diagnostics,
        )?;
        equal(&self.name, "result", &expected.result, &actual.result)?;
        equal(&self.name, "output", &expected.output, &actual.output)
    }
}

pub fn round_trips<T>(original: &T) -> Result<[T; 4], String>
where
    T: Debug + PartialEq + Serialize + DeserializeOwned,
{
    let binary = postcard::to_allocvec(original).map_err(|error| error.to_string())?;
    let pretty = ron::ser::PrettyConfig::default();
    let ron =
        ron::ser::to_string_pretty(original, pretty.clone()).map_err(|error| error.to_string())?;
    let (from_binary, remaining) =
        postcard::take_from_bytes::<T>(&binary).map_err(|error| error.to_string())?;
    if !remaining.is_empty() {
        return Err("round trip left unread binary data".into());
    }
    let from_ron = ron::from_str::<T>(&ron).map_err(|error| error.to_string())?;
    let binary_to_ron = ron::ser::to_string_pretty(&from_binary, pretty.clone())
        .map_err(|error| error.to_string())?;
    let binary_to_ron = ron::from_str::<T>(&binary_to_ron).map_err(|error| error.to_string())?;
    let ron_to_binary = postcard::to_allocvec(&from_ron).map_err(|error| error.to_string())?;
    let (ron_to_binary, remaining) =
        postcard::take_from_bytes::<T>(&ron_to_binary).map_err(|error| error.to_string())?;
    if !remaining.is_empty() {
        return Err("conversion left unread binary data".into());
    }
    let results = [from_binary, from_ron, binary_to_ron, ron_to_binary];
    for result in &results {
        equal("round trip", "logical value", original, result)?;
        let encoded = postcard::to_allocvec(result).map_err(|error| error.to_string())?;
        equal("round trip", "deterministic binary", &binary, &encoded)?;
        let encoded = ron::ser::to_string_pretty(result, pretty.clone())
            .map_err(|error| error.to_string())?;
        equal("round trip", "deterministic RON", &ron, &encoded)?;
    }
    Ok(results)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TraceEvent<Phase, Event> {
    pub time_fs: u64,
    pub delta: u64,
    pub phase: Phase,
    pub event: Event,
}

pub fn normalized_trace<Phase, Event>(
    resolution_fs: u64,
    events: impl IntoIterator<Item = (u64, u64, Phase, Event)>,
) -> Result<Vec<TraceEvent<Phase, Event>>, String> {
    if resolution_fs == 0 {
        return Err("trace resolution must be nonzero".into());
    }
    let mut trace = Vec::new();
    let mut previous = None;
    for (ticks, delta, phase, event) in events {
        let time_fs = ticks
            .checked_mul(resolution_fs)
            .ok_or("trace time conversion overflow")?;
        if previous.is_some_and(|position| position > (time_fs, delta)) {
            return Err("trace time/delta order regressed".into());
        }
        previous = Some((time_fs, delta));
        trace.push(TraceEvent {
            time_fs,
            delta,
            phase,
            event,
        });
    }
    Ok(trace)
}
