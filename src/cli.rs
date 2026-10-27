mod artifact;

use std::{ffi::OsString, io, io::Write, path::PathBuf};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

use crate::artifacts::ArtifactFormat;

#[derive(Parser)]
#[command(
    name = "osv",
    about = "VHDL 2008 simulator and artifact tools (stages not implemented yet)",
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
    #[command(about = "Inspect or convert binary and RON artifacts")]
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
    command: ArtifactCommand,
}

#[derive(Subcommand)]
enum ArtifactCommand {
    #[command(about = "Inspect an artifact as readable RON")]
    Inspect(InspectArguments),
    #[command(about = "Convert binary and RON artifacts without reanalysis")]
    Convert(ConvertArguments),
}

#[derive(Args)]
struct InspectArguments {
    input: PathBuf,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Binary,
    Ron,
}

impl From<OutputFormat> for ArtifactFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Binary => Self::Binary,
            OutputFormat::Ron => Self::Ron,
        }
    }
}

#[derive(Args)]
struct ConvertArguments {
    input: PathBuf,
    #[arg(long, value_enum)]
    to: OutputFormat,
    #[arg(long)]
    output: PathBuf,
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
            ArtifactCommand::Inspect(arguments) => {
                return artifact::inspect(&arguments.input, output, errors);
            }
            ArtifactCommand::Convert(arguments) => {
                return artifact::convert(
                    &arguments.input,
                    &arguments.output,
                    arguments.to.into(),
                    errors,
                );
            }
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
