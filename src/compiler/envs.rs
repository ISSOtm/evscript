use std::{
	collections::{hash_map::Entry, HashMap},
	num::NonZeroU16,
};

use evscript::parsing::{
	AliasParam, DefKind, DefParam, EnvStatement, EnvStatementKind, Ident, Root,
};
use string_interner::StringInterner;

use super::{Diagnostic, FileDb};

#[derive(Debug)]
pub struct Env<'input> {
	pub(crate) funcs: Funcs<'input>,
	// It seems unlikely that an environment's var pool will grow as large as the GB's address space...
	pub(crate) var_pool_size: u16,
}

type Funcs<'input> = HashMap<Ident, Func<'input>>;

#[derive(Debug)]
pub struct Func<'input> {
	pub(crate) args: Vec<DefParam>,
	pub(crate) kind: FuncKind<'input>,
}

#[derive(Debug, Clone)]
pub enum FuncKind<'input> {
	Normal {
		id: u8,
	},
	Alias {
		target: Ident,
		target_args: Vec<AliasParam<'input>>,
	},
	Macro {
		target: Ident,
	},
}

pub fn collect_envs<'input>(
	files: &'input FileDb,
	root_path: &str,
	idents: &StringInterner,
) -> Result<HashMap<Ident, Env<'input>>, Diagnostic> {
	let mut envs = HashMap::new();

	for root in files.iter_roots(root_path.to_owned()) {
		match root {
			// Ignored for this pass.
			Root::Script(_)
			| Root::RawAsm(_)
			| Root::Include(_)
			| Root::Typedef(_)
			| Root::Struct(_) => continue,

			Root::Env(env_stmt) => {
				// Usually, all statements are func defs, except for the pool size.
				// (`use` may throw a wrench into that, but, eh.)
				let mut funcs = HashMap::with_capacity(env_stmt.body.len() - 1);
				let mut pool_size = None;
				let mut id = 0;

				for stmt in &env_stmt.body {
					process_stmt(stmt, idents, &envs, &mut funcs, &mut pool_size, &mut id)?;
				}

				match envs.entry(env_stmt.name) {
					Entry::Occupied(entry) => {
						// TODO: report the location of both definitions
						return Err(Diagnostic::error().with_message(format!(
							"Redefinition of environment {}",
							idents.resolve(env_stmt.name).unwrap()
						)));
					}

					Entry::Vacant(entry) => {
						entry.insert(Env {
							funcs,
							var_pool_size: match pool_size {
								Some(size) => size.get() - 1,
								None => 0,
							},
						});
					}
				}
			}
		}
	}

	Ok(envs)
}

fn process_stmt<'input>(
	stmt: &EnvStatement<'input>,
	idents: &StringInterner,
	envs: &HashMap<Ident, Env<'input>>,
	funcs: &mut Funcs<'input>,
	pool_size: &mut Option<NonZeroU16>,
	id: &mut u8,
) -> Result<(), Diagnostic> {
	match &stmt.kind {
		EnvStatementKind::Def { name, args, kind } => {
			let func_kind = match kind {
				DefKind::Simple => {
					let func_id = *id;
					*id += 1;

					FuncKind::Normal { id: func_id }
				}
				DefKind::Alias {
					target,
					target_args,
				} => FuncKind::Alias {
					target: *target,
					target_args: target_args.to_owned(),
				},
				DefKind::Macro { target } => FuncKind::Macro { target: *target },
			};

			define_func(funcs, *name, args, func_kind, idents)?;
		}

		EnvStatementKind::Use { target } => {
			let target_env = envs.get(target).ok_or_else(|| {
				Diagnostic::error().with_message(format!(
					"Unknown environment \"{}\"",
					idents.resolve(*target).unwrap()
				))
			})?;

			for (name, Func { args, kind }) in &target_env.funcs {
				define_func(funcs, *name, args, kind.clone(), idents)?;
			}
		}

		EnvStatementKind::Pool { size } => {
			let size = size.const_eval().ok_or_else(|| {
				Diagnostic::error()
					.with_message("This pool size is not computable at compile time!")
			})?;

			const MAX_SIZE: i64 = (u16::MAX - 1) as i64;
			if size > MAX_SIZE {
				return Err(Diagnostic::error().with_message(format!(
					"A pool size of {size} is larger than the maximum of {}",
					MAX_SIZE
				)));
			} else if size < 0 {
				return Err(Diagnostic::error().with_message("A pool size cannot be negative"));
			}
			let size = size as u16;

			if pool_size.is_some() {
				return Err(Diagnostic::error().with_message("Redefinition of the pool size"));
			}
			*pool_size = Some(NonZeroU16::new(size + 1).unwrap());
		}
	}

	Ok(())
}

fn define_func<'input>(
	funcs: &mut Funcs<'input>,
	name: Ident,
	args: &[DefParam],
	kind: FuncKind<'input>,
	idents: &StringInterner,
) -> Result<(), Diagnostic> {
	if let FuncKind::Normal { id } = &kind {
		// Check that the function's ID doesn't clash.
		for (other_name, func) in &*funcs {
			if let FuncKind::Normal { id: other_id } = &func.kind {
				if id == other_id {
					return Err(Diagnostic::error().with_message(format!(
						"Function {} tries to have the same ID ({id}) as {}",
						idents.resolve(name).unwrap(),
						idents.resolve(*other_name).unwrap(),
					)));
				}
			}
		}
	}

	match funcs.entry(name) {
		Entry::Occupied(_) => Err(Diagnostic::error().with_message(format!(
			"Redefinition of function {}",
			idents.resolve(name).unwrap(),
		))),

		Entry::Vacant(entry) => {
			entry.insert(Func {
				args: args.to_vec(),
				kind,
			});

			Ok(())
		}
	}
}
