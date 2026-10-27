use std::{ffi::OsString, io, io::Write};

use clap::{Args, CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "osv",
    about = "VHDL 2008 simulator bootstrap (commands not implemented yet)",
    version = concat!(env!("CARGO_PKG_VERSION"), " (bootstrap)"),
)]
struct Cli {
    #[command(subcommand)]
    command: Option<StageCommand>,
}

#[derive(Subcommand)]
enum StageCommand {
    #[command(about = "Analyze VHDL sources (not implemented yet)")]
    Analyze(PendingArguments),
    #[command(about = "Elaborate analyzed artifacts (not implemented yet)")]
    Elaborate(PendingArguments),
    #[command(about = "Simulate an elaborated artifact (not implemented yet)")]
    Simulate(PendingArguments),
    #[command(about = "Inspect or convert artifacts (not implemented yet)")]
    Artifact(ArtifactArguments),
}

#[derive(Args)]
struct PendingArguments {
    #[arg(
        value_name = "ARGUMENTS",
        trailing_var_arg = true,
        allow_hyphen_values = true
    )]
    arguments: Vec<OsString>,
}

#[derive(Args)]
struct ArtifactArguments {
    #[command(subcommand)]
    command: Option<ArtifactCommand>,
}

#[derive(Subcommand)]
enum ArtifactCommand {
    #[command(about = "Inspect an artifact as readable RON (not implemented yet)")]
    Inspect(PendingArguments),
    #[command(about = "Convert binary and RON artifacts (not implemented yet)")]
    Convert(PendingArguments),
}

pub fn run(
    arguments: impl IntoIterator<Item = OsString>,
    output: &mut impl Write,
    errors: &mut impl Write,
) -> io::Result<u8> {
    let cli = match Cli::try_parse_from(std::iter::once(OsString::from("osv")).chain(arguments)) {
        Ok(cli) => cli,
        Err(error) if !error.use_stderr() => {
            write!(output, "{error}")?;
            return Ok(0);
        }
        Err(error) => {
            write!(errors, "error[OSV-CLI-USAGE]: {error}")?;
            return Ok(2);
        }
    };

    let command = match cli.command {
        Some(StageCommand::Analyze(_)) => "analyze",
        Some(StageCommand::Elaborate(_)) => "elaborate",
        Some(StageCommand::Simulate(_)) => "simulate",
        Some(StageCommand::Artifact(arguments)) => match arguments.command {
            Some(ArtifactCommand::Inspect(_)) => "artifact inspect",
            Some(ArtifactCommand::Convert(_)) => "artifact convert",
            None => "artifact",
        },
        None => {
            Cli::command().write_long_help(output)?;
            writeln!(output)?;
            return Ok(0);
        }
    };
    writeln!(
        errors,
        "error[OSV-CLI-UNIMPLEMENTED]: command {command:?} is not implemented yet"
    )?;
    Ok(2)
}
