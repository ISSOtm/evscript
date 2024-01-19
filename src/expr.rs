use std::ops::Index;

use crate::parse::Ident;

#[derive(Debug, Clone)]
pub struct Expr<'input> {
	/// This array of operations works similarly to RPN: an [`Op`] in this vector "consumes" one or more values, and produces exactly one.
	/// An operation's value can be referenced using an [`OpRef`].
	ops: Vec<Op<'input>>,
}

/// An index into an [`Expr`]'s vector of [`Op`]s.
/// This is used to let [`Op`]s reference other [`Op`]s (as operands).
#[derive(Debug, Clone, Copy)]
pub struct OpRef(usize);

#[derive(Debug, Clone)]
pub enum Op<'input> {
	// Terminals.
	Number(i64),
	String(&'input str),
	Variable(Ident),
	Address(Ident),
	Call(Ident, Vec<OpRef>),

	// Unary operators.
	Deref(OpRef),
	Neg(OpRef),
	Cpl(OpRef),

	// Binary operators.
	LogicalOr(OpRef, OpRef),
	LogicalAnd(OpRef, OpRef),
	Equ(OpRef, OpRef),
	NotEqu(OpRef, OpRef),
	LessThan(OpRef, OpRef),
	LessThanEqu(OpRef, OpRef),
	GreaterThan(OpRef, OpRef),
	GreaterThanEqu(OpRef, OpRef),
	BinaryOr(OpRef, OpRef),
	BinaryXor(OpRef, OpRef),
	BinaryAnd(OpRef, OpRef),
	ShiftLeft(OpRef, OpRef),
	ShiftRight(OpRef, OpRef),
	Add(OpRef, OpRef),
	Sub(OpRef, OpRef),
	Mul(OpRef, OpRef),
	Div(OpRef, OpRef),
	Mod(OpRef, OpRef),
}

impl<'input> Expr<'input> {
	pub fn address(variable: Ident) -> Self {
		Self {
			ops: vec![Op::Address(variable)],
		}
	}

	pub fn func_call(func: Ident, mut args: Vec<Expr<'input>>) -> Self {
		// Compute the indices of the arguments in the concatenated vector.
		let mut acc = 0;
		let arg_refs = args
			.iter()
			.map(|arg| {
				acc += arg.ops.len();
				OpRef(acc - 1)
			})
			.collect();
		// Generate the `call` operation, for later. (This collects two locals into a single object.)
		let call = Op::Call(func, arg_refs);

		// `split_at_mut(1)` would panic if `args` was empty.
		let ops = if args.is_empty() {
			vec![call]
		} else {
			// Concatenate all of the operations.
			let (first, rest) = args.split_at_mut(1);
			let first = &mut first[0].ops;
			first.reserve(acc - first.len() + 1);
			for arg in rest {
				first.append(&mut arg.ops);
			}

			// Ideally we'd just move out of `args[0]` to avoid the overhead of `swap_remove`, but
			// apparently this doesn't work. (Why?)
			// `swap_remove` is simpler than `remove()` or `drain()`, so I expect it should be more
			// optimizer-friendly.
			args.swap_remove(0).ops
		};
		Self { ops }
	}

	pub fn unary_op<Oper: FnOnce(OpRef) -> Op<'input>, ConstEval: FnOnce(i64) -> i64>(
		mut expr: Self,
		operator: Oper,
		const_eval: ConstEval,
	) -> Self {
		if let [Op::Number(n)] = &mut expr.ops[..] {
			*n = const_eval(*n);
		} else {
			let idx = OpRef(expr.ops.len());
			expr.ops.push(operator(idx));
		}
		expr
	}

	// This one cannot be constant-evaluated, so it can't use [`unary_op`][Self::unary_op].
	pub fn deref(mut expr: Self) -> Self {
		let idx = OpRef(expr.ops.len());
		expr.ops.push(Op::Deref(idx));
		expr
	}

	pub fn binary_op<
		Oper: FnOnce(OpRef, OpRef) -> Op<'input>,
		ConstEval: FnOnce(i64, i64) -> i64,
	>(
		mut lhs: Self,
		mut rhs: Self,
		operator: Oper,
		const_eval: ConstEval,
	) -> Self {
		if let ([Op::Number(lhs)], [Op::Number(rhs)]) = (&mut lhs.ops[..], &rhs.ops[..]) {
			*lhs = const_eval(*lhs, *rhs);
		} else {
			let left_idx = OpRef(lhs.ops.len());
			let right_idx = OpRef(rhs.ops.len());
			lhs.ops.reserve(rhs.ops.len() + 1);
			lhs.ops.append(&mut rhs.ops);
			lhs.ops.push(operator(left_idx, right_idx));
		}
		lhs
	}
}

impl<'input> Index<OpRef> for Expr<'input> {
	type Output = Op<'input>;

	fn index(&self, index: OpRef) -> &Self::Output {
		&self.ops[index.0]
	}
}

/// This allows creating [`Expr`]s from terminals (numbers, etc.) without writing much boilerplate.
impl<'input, T> From<T> for Expr<'input>
where
	Op<'input>: From<T>,
{
	fn from(value: T) -> Self {
		Self {
			ops: vec![value.into()],
		}
	}
}

impl From<i64> for Op<'_> {
	fn from(value: i64) -> Self {
		Self::Number(value)
	}
}
impl<'input> From<&'input str> for Op<'input> {
	fn from(value: &'input str) -> Self {
		Self::String(value)
	}
}
impl From<Ident> for Op<'_> {
	fn from(value: Ident) -> Self {
		Self::Variable(value)
	}
}
