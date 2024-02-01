use std::process::ExitCode;

use clap::Parser;
use codespan_reporting::{diagnostic::Diagnostic, term::termcolor::ColorChoice};
use string_interner::StringInterner;

mod compiler;
use compiler::{DiagReporter, FileDb};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
	/// Output file
	#[clap(short, long, value_parser, value_name = "PATH")]
	output: String,

	/// Report the peak memory usage of each function
	#[clap(long = "report-usage")]
	report_usage: bool,

	/// Input file
	#[clap(value_parser, value_name = "PATH")]
	input: String,
}

fn main() -> ExitCode {
	let cli = Cli::parse();
	let mut err_reporter = DiagReporter::new(ColorChoice::Auto); // TODO

	let mut files = FileDb::new();
	let mut idents = StringInterner::new();

	if let Err(diag) = compile(cli, &mut files, &mut idents, &mut err_reporter) {
		err_reporter.emit(&files, &diag);
		return ExitCode::FAILURE;
	}

	todo!();
}

fn compile(
	cli: Cli,
	files: &mut FileDb,
	idents: &mut StringInterner,
	err_reporter: &mut DiagReporter,
) -> Result<(), Diagnostic<&'static str>> {
	files.parse_files(&cli.input, idents, err_reporter)?;

	let types = compiler::collect_types(files, &cli.input, idents)?;

	todo!();
}
