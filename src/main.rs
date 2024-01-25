use std::{
	collections::{hash_map::Entry, HashMap},
	fmt::Display,
	ops::Deref,
};

use clap::Parser;
use codespan_reporting::{
	diagnostic::{Diagnostic, Label},
	files::{Files, SimpleFile},
	term::termcolor::ColorChoice,
};
use evscript::parsing::{ParseError, Root};
use string_interner::StringInterner;
use yoke::{Yoke, Yokeable};

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

/// A "database" storing the source code of each input file, as well as some info cached from that.
#[derive(Debug)]
struct FileDb {
	files: HashMap<String, (Yoke<Roots<'static>, String>, Vec<usize>)>,
}

impl FileDb {
	fn new() -> Self {
		Self {
			files: HashMap::new(),
		}
	}

	/// Loads an evscript source file from the filesystem.
	///
	/// If the loading fails for any reason, this function prints the error and then dies.
	fn load_or_die(
		&mut self,
		path: &str,
		idents: &mut StringInterner,
		err_reporter: &mut DiagReporter,
	) -> Result<&Roots<'_>, std::io::Error> {
		Ok(match self.files.entry(path.to_owned()) {
			// If the file has already been loaded, don't do the work again.
			Entry::Occupied(entry) => entry.into_mut(),

			// If the file has not been loaded it again, load it and cache the returned AST.
			Entry::Vacant(entry) => {
				// Try reading the source code. Die if that fails for any reason.
				let source = match std::fs::read_to_string(entry.key()) {
					Ok(source) => source,
					Err(err) => {
						let diag = Diagnostic::error()
							.with_message(format!("Failed to read input file \"{path}\": {err}"));
						err_reporter.emit(&DummyFiles, &diag);
						std::process::exit(1);
					}
				};

				// Since the parsing result borrows from the source code string, we need to use a `Yoke`.
				// We need to keep the entire source code around for reporting errors with source
				// code, so might as well avoid copies, huh?
				let yoke = Yoke::attach_to_cart(source, |source| {
					// The syntax error must be reported immediately, as `ParseError`s borrow from
					// the `source`, but `Yoke`'s API cannot accomodate that.
					let roots = match evscript::parsing::parse(source, idents) {
						Ok(roots) => roots,
						Err(parse_err) => {
							let file = SimpleFile::new(path, source);
							err_reporter.emit_parse_error(&file, parse_err);
							std::process::exit(1);
						}
					};
					Roots(roots)
				});

				// Compute the byte indices at which lines start; this significantly speeds up
				// error reporting.
				// TODO: only compute this when requested, instead of every time a file is loaded?
				let line_starts =
					codespan_reporting::files::line_starts(yoke.backing_cart()).collect();

				// Now that we have all the components, insert the entry!
				entry.insert((yoke, line_starts))
			}
		}
		.0
		.get())
	}

	fn get(
		&self,
		path: &str,
	) -> Result<&(Yoke<Roots<'static>, String>, Vec<usize>), codespan_reporting::files::Error> {
		self.files
			.get(path)
			.ok_or(codespan_reporting::files::Error::FileMissing)
	}
}

impl<'a> Files<'a> for FileDb {
	type FileId = &'a str;
	type Name = &'a str;
	type Source = &'a str;

	fn name(&'a self, id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
		Ok(id)
	}

	fn source(
		&'a self,
		id: Self::FileId,
	) -> Result<Self::Source, codespan_reporting::files::Error> {
		self.get(id)
			.map(|(source, _line_starts)| source.backing_cart().as_str())
	}

	fn line_index(
		&'a self,
		id: Self::FileId,
		byte_index: usize,
	) -> Result<usize, codespan_reporting::files::Error> {
		self.get(id).map(|(_source, line_starts)| {
			line_starts
				.binary_search(&byte_index)
				.unwrap_or_else(|next_line| next_line - 1)
		})
	}

	fn line_range(
		&'a self,
		id: Self::FileId,
		line_index: usize,
	) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
		self.get(id)
			.and_then(|(source, line_starts)| match line_starts.get(line_index) {
				Some(&start) => {
					let end = match line_starts.get(line_index + 1) {
						Some(&end) => end,
						None => source.backing_cart().len(),
					} - 1;
					Ok(start..end)
				}
				None => Err(codespan_reporting::files::Error::LineTooLarge {
					given: line_index,
					max: line_starts.len(),
				}),
			})
	}
}

/// A newtype to store the result of parsing; this is required because we need it to implement
/// [`Yokeable`].
#[derive(Debug, Yokeable)]
struct Roots<'a>(Vec<Root<'a>>);

impl<'a> Deref for Roots<'a> {
	type Target = Vec<Root<'a>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// A convenience struct to group all of the code and data related to printing diagnostics.
#[derive(Debug)]
struct DiagReporter {
	output: codespan_reporting::term::termcolor::StandardStream,
	config: codespan_reporting::term::Config,
}

impl DiagReporter {
	fn new(color_choice: ColorChoice) -> Self {
		let stderr = codespan_reporting::term::termcolor::StandardStream::stderr(color_choice);
		Self {
			output: stderr,
			config: Default::default(), // TODO
		}
	}

	/// Emits a [`Diagnostic`].
	fn emit<'files, F: Files<'files>>(
		&mut self,
		files: &'files F,
		diagnostic: &Diagnostic<<F as Files<'files>>::FileId>,
	) {
		codespan_reporting::term::emit(&mut self.output, &self.config, files, diagnostic).unwrap()
	}

	/// Emits an error that occurs when trying to parse an input file.
	fn emit_parse_error<'files, F: Files<'files, FileId = ()>>(
		&mut self,
		files: &'files F,
		parse_err: ParseError<'_>,
	) {
		let mut diag = Diagnostic::error();
		match parse_err {
			ParseError::InvalidToken { location } => {
				diag.message = "Invalid token".to_string();
				diag.labels = vec![Label::primary((), location..location)];
			}
			ParseError::UnrecognizedEof { location, expected } => {
				diag.message = "Unexpected end of file".to_string();
				diag.labels = vec![Label::primary((), location..location)];
				diag.notes = vec![format!("Expected {}", ExpectedTokList(expected))];
			}
			ParseError::UnrecognizedToken {
				token: (start, token, end),
				expected,
			} => {
				diag.message = format!("Unexpected {token}");
				diag.labels = vec![Label::primary((), start..end)];
				diag.notes = vec![format!("Expected {}", ExpectedTokList(expected))];
			}
			ParseError::ExtraToken {
				token: (start, token, end),
			} => {
				diag.message = format!("This {token} should not be here");
				diag.labels = vec![Label::primary((), start..end)];
				diag.notes = vec!["Expected no more tokens".to_string()];
			}
			ParseError::User { error } => diag.message = error.to_string(),
		}
		self.emit(files, &diag)
	}
}

/// A convenience struct for printing a list of expected token's names.
#[derive(Debug)]
struct ExpectedTokList(Vec<String>);

impl Display for ExpectedTokList {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let expected = &self.0;
		if let [only_token] = &expected[..] {
			write!(f, "{only_token}")
		} else {
			write!(f, "one of {}", expected[0])?;
			for token in &expected[1..expected.len() - 1] {
				write!(f, ", {token}")?;
			}
			write!(f, ", or {}", &expected[expected.len() - 1])
		}
	}
}

/// This is used for diagnostics that don't contain any labels, because `codespan_reporting` requires
/// a `Files` backend always.
#[derive(Debug)]
struct DummyFiles;

impl<'a> Files<'a> for DummyFiles {
	type FileId = ();
	type Name = &'a str;
	type Source = &'a str;

	fn name(&'a self, _id: Self::FileId) -> Result<Self::Name, codespan_reporting::files::Error> {
		unreachable!();
	}

	fn source(
		&'a self,
		_id: Self::FileId,
	) -> Result<Self::Source, codespan_reporting::files::Error> {
		unreachable!();
	}

	fn line_index(
		&'a self,
		_id: Self::FileId,
		_byte_indexx: usize,
	) -> Result<usize, codespan_reporting::files::Error> {
		unreachable!();
	}

	fn line_range(
		&'a self,
		_id: Self::FileId,
		_line_index: usize,
	) -> Result<std::ops::Range<usize>, codespan_reporting::files::Error> {
		unreachable!();
	}
}
