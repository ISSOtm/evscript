use std::collections::HashMap;

use codespan_reporting::diagnostic::Diagnostic;
use evscript::parsing::{Ident, Root};
use string_interner::StringInterner;

use super::FileDb;

#[derive(Debug)]
pub enum Type {
	Primitive { signed: bool, sizeof: u8 },
	Alias(Ident),
	Struct(Vec<StructMember>),
}

#[derive(Debug)]
pub struct StructMember {
	name: Ident,
	r#type: Ident,
}

pub fn collect_types(
	files: &mut FileDb,
	root_path: &str,
	idents: &mut StringInterner,
) -> Result<HashMap<Ident, Type>, Diagnostic<&'static str>> {
	let mut types = HashMap::new();
	types.insert(
		idents.get_or_intern_static("u8"),
		Type::Primitive {
			signed: false,
			sizeof: 1,
		},
	);
	types.insert(
		idents.get_or_intern_static("u16"),
		Type::Primitive {
			signed: false,
			sizeof: 2,
		},
	);

	for root in files.iter_roots(root_path.to_owned()) {
		let (name, kind) = match root {
			// Ignored for this pass.
			Root::Script(_) | Root::Env(_) | Root::RawAsm(_) | Root::Include(_) => continue,

			Root::Typedef(typedef) => (typedef.name, Type::Alias(typedef.target)),
			Root::Struct(struct_def) => (
				struct_def.name,
				Type::Struct(struct_def.members.iter().map(Into::into).collect()),
			),
		};

		// TODO: report location of both definitions
		if let Some(other) = types.insert(name, kind) {
			return Err(Diagnostic::error().with_message(format!(
				"Redefinition of type {}",
				idents.resolve(name).unwrap()
			)));
		}
	}

	Ok(types)
}

impl From<&evscript::parsing::StructMember> for StructMember {
	fn from(value: &evscript::parsing::StructMember) -> Self {
		Self {
			name: value.name,
			r#type: value.r#type,
		}
	}
}
