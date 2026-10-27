#![forbid(unsafe_code)]

use std::{io, io::Write, process::ExitCode};

fn main() -> ExitCode {
    match osv::cli::run(
        std::env::args_os().skip(1),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ) {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            let _ = writeln!(io::stderr(), "error[OSV-CLI-IO]: {error}");
            ExitCode::from(2)
        }
    }
}
