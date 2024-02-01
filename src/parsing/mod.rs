use lalrpop_util::lalrpop_mod;
use string_interner::StringInterner;

use crate::expr::{Expr, Op, OpRef};

lalrpop_mod!(parser, "/parsing/parser.rs");

/// An identifier in evscript.
pub type Ident = string_interner::symbol::SymbolU32;
/// The location type used to refer to places in the source code.
pub type Location = usize; // LALRPOP's built-in lexer returns `usize`s, and using a newtype would incur a *lot* of boilerplate.
/// A reference to a slice of the source code.
pub type Span = (Location, Location);
/// This alias type is required to work around LALRPOP not supporting `fn()` syntax for token types.
type OpConstructor<'input> = fn(OpRef, OpRef) -> Op<'input>;

/// Attempts to parse a string as evscript source code.
pub fn parse<'input>(
	input: &'input str,
	identifiers: &mut StringInterner,
) -> Result<Vec<Root<'input>>, ParseError<'input>> {
	parser::FileParser::new().parse(identifiers, input)
}

/// An error that can cause evscript parsing to fail.
pub type ParseError<'input> =
	lalrpop_util::ParseError<Location, lalrpop_util::lexer::Token<'input>, &'static str>;

/// The kinds of statements can exist at the root of an evscript file.
#[derive(Debug, Clone)]
pub enum Root<'input> {
	Script(Script<'input>),
	Env(Env<'input>),
	RawAsm(RawAsm<'input>),
	Include(Include<'input>),
	Typedef(Typedef),
	Struct(Struct),
}

/// A "script" block.
#[derive(Debug, Clone)]
pub struct Script<'input> {
	/// The source span that encompasses the script's type and name.
	pub span: Span,
	/// The script's type, which should name an environment.
	pub r#type: Ident,
	/// The script's name.
	pub name: Ident,
	/// Statements contained within the script.
	pub body: Vec<ScriptStatement<'input>>,
}

/// A statement that can occur within a script.
#[derive(Debug, Clone)]
pub struct ScriptStatement<'input> {
	/// The source span that encompasses the statement, trailing semicolon not included.
	pub span: Span,
	/// The statement's payload.
	pub kind: ScriptStatementKind<'input>,
}

/// The kinds of statements that can occur within a script block.
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

/// Statements that can be used in more than "root" contexts (for example, `for` init).
#[derive(Debug, Clone)]
pub enum SimpleStatement<'input> {
	/// Simple evaluation of an expression.
	///
	/// (This is mostly useful for "naked" function calls.)
	Expr(Expr<'input>),
	/// A variable declaration.
	VarDecl {
		/// The variable's name.
		name: Ident,
		/// The variable's type.
		r#type: VarType,
		/// The value the variable is initialized with.
		init: Option<Expr<'input>>,
	},
	/// A modification of a variable's value.
	Assignment {
		/// What variable is being modified.
		name: Ident,
		/// What the variable is being set to.
		value: Expr<'input>,
	},
}

/// A variable's type.
#[derive(Debug, Clone)]
pub struct VarType {
	pub base_type: Ident,
	/// If `true`, the type is a pointer to the `base_type`.
	pub is_ptr: bool,
}

#[derive(Debug, Clone)]
pub struct Env<'input> {
	/// The source span that encompasses the env keyword and the env's name.
	pub span: Span,
	pub name: Ident,
	pub body: Vec<EnvStatement<'input>>,
}

#[derive(Debug, Clone)]
pub struct EnvStatement<'input> {
	/// The source span that encompasses the statement, trailing semicolon not included.
	pub span: Span,
	pub kind: EnvStatementKind<'input>,
}

/// The kinds of statements that can occur within an env block.
#[derive(Debug, Clone)]
pub enum EnvStatementKind<'input> {
	/// A function declaration.
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
	/// The function is simply called as such.
	Simple,
	/// The function is an alias for another one, and does not have its own bytecode entry.
	Alias {
		target: Ident,
		target_args: Vec<AliasParam<'input>>,
	},
	// TODO: document this one
	Macro {
		target: Ident,
	},
}

#[derive(Debug, Clone)]
pub struct RawAsm<'input> {
	pub contents: &'input str,
}

#[derive(Debug, Clone)]
pub struct Include<'input> {
	pub path: &'input str,
}

/// A `typedef` statement.
#[derive(Debug, Clone)]
pub struct Typedef {
	/// The name of the type being created.
	pub name: Ident,
	/// What type the alias is referring to.
	pub target: Ident,
}

#[derive(Debug, Clone)]
pub struct Struct {
	pub name: Ident,
	pub members: Vec<StructMember>,
}

#[derive(Debug, Clone)]
pub struct StructMember {
	pub name: Ident,
	pub r#type: Ident,
}

#[derive(Debug, Clone)]
pub struct DefParam {
	pub name: Ident,
	pub kind: DefParamKind,
}

#[derive(Debug, Clone)]
pub enum DefParamKind {
	/// The parameter is a regular parameter, passed to the function as-is.
	Simple,
	/// The parameter is an out parameter.
	Return,
	// TODO: document this
	Const,
}

#[derive(Debug, Clone)]
pub enum AliasParam<'input> {
	/// One of the alias' parameters is passed as-is.
	Placeholder(usize),
	// TODO: document this
	Expr {
		is_const: bool,
		expr: Expr<'input>,
	},
}
