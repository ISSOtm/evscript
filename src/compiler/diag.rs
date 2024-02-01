use std::fmt::Display;

use codespan_reporting::{
	diagnostic::{Diagnostic, Label},
	files::Files,
	term::termcolor::ColorChoice,
};
use evscript::parsing::ParseError;

/// A convenience struct to group all of the code and data related to printing diagnostics.
#[derive(Debug)]
pub struct DiagReporter {
	output: codespan_reporting::term::termcolor::StandardStream,
	config: codespan_reporting::term::Config,
}

impl DiagReporter {
	pub fn new(color_choice: ColorChoice) -> Self {
		let stderr = codespan_reporting::term::termcolor::StandardStream::stderr(color_choice);
		Self {
			output: stderr,
			config: Default::default(), // TODO
		}
	}

	/// Emits a [`Diagnostic`].
	pub fn emit<'files, F: Files<'files>>(
		&mut self,
		files: &'files F,
		diagnostic: &Diagnostic<<F as Files<'files>>::FileId>,
	) {
		codespan_reporting::term::emit(&mut self.output, &self.config, files, diagnostic).unwrap()
	}

	/// Emits an error that occurs when trying to parse an input file.
	pub fn emit_parse_error<'files, F: Files<'files, FileId = ()>>(
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
pub struct DummyFiles;

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
		_byte_index: usize,
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
