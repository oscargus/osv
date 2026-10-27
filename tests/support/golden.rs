use osv::{
    diagnostics::{Diagnostic, render},
    source::SourceMap,
};

pub fn rendered(diagnostic: &Diagnostic, sources: &SourceMap) -> String {
    let mut output = Vec::new();
    render(diagnostic, sources, &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains('\r'));
    assert!(!output.contains('\x1b'));
    output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

pub fn expected_output(snapshot: &str) -> String {
    ron::from_str(&snapshot.replace("\r\n", "\n")).unwrap()
}
