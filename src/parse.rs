use lalrpop_util::lalrpop_mod;

use crate::expr::{Expr, Op, OpRef};

lalrpop_mod!(parser);

#[derive(Debug, Clone, Copy)]
pub struct Ident(string_interner::symbol::SymbolU32);
/// The location type used to refer to the source.
pub type Location = usize; // LALRPOP's built-in lexer returns `usize`s, and using a newtype would incur a *lot* of boilerplate.
/// A reference to a slice of the source code.
pub type Span = (Location, Location);
type OpConstructor<'input> = fn(OpRef, OpRef) -> Op<'input>;

#[derive(Debug, Clone)]
pub enum Root<'input> {
	Script(Script<'input>),
	Env(Env<'input>),
	RawAsm(RawAsm<'input>),
	Include(Include<'input>),
	Typedef(Typedef),
	Struct(Struct),
}

#[derive(Debug, Clone)]
pub struct Script<'input> {
	span: Span,
	r#type: Ident,
	name: Ident,
	body: Vec<ScriptStatement<'input>>,
}

#[derive(Debug, Clone)]
pub struct ScriptStatement<'input> {
	span: Span,
	kind: ScriptStatementKind<'input>,
}

#[derive(Debug, Clone)]
pub enum ScriptStatementKind<'input> {
	SimpleStatement(SimpleStatement<'input>),
	If {
		cond: Expr<'input>,
		body: Vec<ScriptStatement<'input>>,
		else_stmt: Vec<ScriptStatement<'input>>,
	},
	While {
		cond: Expr<'input>,
		body: Vec<ScriptStatement<'input>>,
	},
	DoWhile {
		cond: Expr<'input>,
		body: Vec<ScriptStatement<'input>>,
	},
	For {
		init: SimpleStatement<'input>,
		cond: Expr<'input>,
		body: Vec<ScriptStatement<'input>>,
		post_body: SimpleStatement<'input>,
	},
	Repeat {
		cond: Expr<'input>,
		body: Vec<ScriptStatement<'input>>,
	},
	Loop {
		body: Vec<ScriptStatement<'input>>,
	},
}

#[derive(Debug, Clone)]
pub enum SimpleStatement<'input> {
	Expr(Expr<'input>),
	VarDecl {
		name: Ident,
		r#type: VarType,
		init: Option<Expr<'input>>,
	},
	Assignment {
		name: Ident,
		value: Expr<'input>,
	},
}

#[derive(Debug, Clone)]
pub struct VarType {
	base_type: Ident,
	is_ptr: bool,
}

#[derive(Debug, Clone)]
pub struct Env<'input> {
	name: Ident,
	body: Vec<EnvStatement<'input>>,
}

#[derive(Debug, Clone)]
pub struct EnvStatement<'input> {
	span: Span,
	kind: EnvStatementKind<'input>,
}

#[derive(Debug, Clone)]
pub enum EnvStatementKind<'input> {
	Def {
		name: Ident,
		args: Vec<DefParam>,
		kind: DefKind<'input>,
	},
	Use {
		target: Ident,
	},
	Pool {
		size: Expr<'input>,
	},
}

#[derive(Debug, Clone)]
pub enum DefKind<'input> {
	Simple,
	Alias {
		target: Ident,
		target_args: Vec<AliasParam<'input>>,
	},
	Macro {
		target: Ident,
	},
}

#[derive(Debug, Clone)]
pub struct RawAsm<'input> {
	contents: &'input str,
}

#[derive(Debug, Clone)]
pub struct Include<'input> {
	path: &'input str,
}

#[derive(Debug, Clone)]
pub struct Typedef {
	name: Ident,
	target: Ident,
}

#[derive(Debug, Clone)]
pub struct Struct {
	name: Ident,
	members: Vec<StructMember>,
}

#[derive(Debug, Clone)]
pub struct StructMember {
	name: Ident,
	r#type: Ident,
}

#[derive(Debug, Clone)]
pub struct DefParam {
	name: Ident,
	kind: DefParamKind,
}

#[derive(Debug, Clone)]
pub enum DefParamKind {
	Simple,
	Return,
	Const,
}

#[derive(Debug, Clone)]
pub enum AliasParam<'input> {
	Placeholder(usize),
	Expr { is_const: bool, expr: Expr<'input> },
}
