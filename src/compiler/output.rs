use std::{collections::HashMap, fmt::Display, fs::File, io::Write};

use evscript::parsing::Ident;
use string_interner::StringInterner;

use crate::{compiler::FuncKind, Cli};

use super::{Diagnostic, Env, FileDb, Type};

pub fn emit(
	cli: &Cli,
	idents: &StringInterner,
	types: HashMap<Ident, Type>,
	envs: HashMap<Ident, Env>,
) -> Result<(), Diagnostic> {
	let mut output = File::create(&cli.output).map_err(|err| {
		Diagnostic::error().with_message(format!(
			"Failed to open or create output file \"{}\": {err}",
			cli.output
		))
	})?;
	let diag_emit = |io_res: std::io::Result<()>| {
		io_res.map_err(|err| {
			Diagnostic::error().with_message(format!(
				"Failed to write to output file \"{}\": {err}",
				cli.output
			))
		})
	};
	macro_rules! emit {
        ($($what:tt)*) => { diag_emit(writeln!(output, $($what)*))? };
    }
	macro_rules! explain {
        ($($what:tt)*) => { if cli.explain { emit!($($what)*); } };
    }

	// First, emit the envs.
	for (env_ident, env) in &envs {
		let env_name = idents.resolve(*env_ident).unwrap();

		explain!("; env {env_name} {{");
		explain!(";     pool = {}", env.var_pool_size);
		for (func_ident, func) in &env.funcs {
			match &func.kind {
				FuncKind::Normal { id } => {
					emit!(
						"def {} equ {id}",
						FuncName::new_from_idents(*env_ident, *func_ident, idents)
					);
				}
				FuncKind::Alias {
					target,
					target_args,
				} => {} // Does not emit anything by itself.
				FuncKind::Macro { target } => {} // TODO: check that the macro exists, warn otherwise?
			}
		}
		explain!("; }}");
	}

	// Now, everything else.
	todo!();
}

/// Convenience object for displaying function names in a consistent manner.
#[derive(Debug)]
struct FuncName<'env, 'func> {
	env_name: &'env str,
	func_name: &'func str,
}

impl FuncName<'_, '_> {
	fn new_from_idents(
		env_name: Ident,
		func_name: Ident,
		idents: &StringInterner,
	) -> FuncName<'_, '_> {
		FuncName {
			env_name: idents.resolve(env_name).unwrap(),
			func_name: idents.resolve(func_name).unwrap(),
		}
	}
}

impl Display for FuncName<'_, '_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}__{}", self.env_name, self.func_name)
	}
}
