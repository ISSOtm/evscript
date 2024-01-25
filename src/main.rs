use clap::Parser;
use codespan_reporting::term::termcolor::ColorChoice;
use string_interner::StringInterner;

use crate::compiler::*;

mod compiler;

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

fn main() {
	let cli = Cli::parse();
	let mut err_reporter = DiagReporter::new(ColorChoice::Auto); // TODO

	let mut files = FileDb::new();
	let mut idents = StringInterner::new();

	let roots = files.load_or_die(&cli.input, &mut idents, &mut err_reporter);

	todo!();
}
