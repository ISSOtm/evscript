use std::{
    collections::{hash_map::Entry, HashMap}, ops::Deref};

use codespan_reporting::{
    diagnostic::Diagnostic, files::{SimpleFile, Files}};
use evscript::parsing::Root;
use string_interner::StringInterner;
use yoke::{Yoke, Yokeable};

use super::{diag::DummyFiles, DiagReporter};

/// A "database" storing the source code of each input file, as well as some info cached from that.
#[derive(Debug)]
pub struct FileDb {
    files: HashMap<String, (Yoke<Roots<'static>, String>, Vec<usize>)>,
}

impl FileDb {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Loads an evscript source file from the filesystem.
    ///
    /// If the loading fails for any reason, this function prints the error and then dies.
    pub fn load_or_die(
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
pub struct Roots<'a>(Vec<Root<'a>>);

impl<'a> Deref for Roots<'a> {
    type Target = Vec<Root<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
