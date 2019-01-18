use super::super::primitives::proc::Procedure::{Basic, Ctx};
use super::super::{Context, Error, Primitive, Result};
use super::SExp::{self, Atom, Null, Pair, Vector};

mod helpers;
mod tests;

/// Evaluate one or more S-Expressions, in the base context or your own custom one.
///
/// # Examples
/// ```
/// use parsley::prelude::*;
/// assert!(eval!(sexp!["potato", true, 5]).is_ok());
/// assert!(eval!(sexp![SExp::sym("potato"), true, 5]).is_err());
/// ```
/// ```
/// use parsley::prelude::*;
///
/// let evaluated = eval!(
///     sexp![
///         SExp::sym("define"),
///         sexp![SExp::sym("square"), SExp::sym("x")],
///         sexp![SExp::sym("*"), SExp::sym("x"), SExp::sym("x")]
///     ],
///     sexp![SExp::sym("square"), 12]
/// );
///
/// assert!(evaluated.is_ok());
/// assert_eq!(evaluated.unwrap(), SExp::from(144));
/// ```
/// ```
/// use parsley::prelude::*;
/// let mut ctx = Context::base();
///
/// eval!(
///     ctx;
///     sexp![
///         SExp::sym("define"),
///         sexp![SExp::sym("potato"), SExp::sym("x"), SExp::sym("y")],
///         SExp::sym("y")
///     ]
/// );
/// let evaluated = eval!(ctx; sexp![SExp::sym("potato"), true, 5]);
///
/// assert!(evaluated.is_ok());
/// assert_eq!(evaluated.unwrap(), SExp::from(5));
/// ```
#[macro_export]
macro_rules! eval {
    ( $( $expression:expr ),* ) => {
        $crate::eval!($crate::Context::base(); $( $expression ),*)
    };
    ( $context:expr; $expression:expr ) => {
        $expression.eval(&mut $context)
    };
    ( $context:expr; $( $expression:expr ),* ) => {
        $crate::sexp![
            $crate::SExp::sym("begin"),
            $( $expression ),*
        ].eval(&mut $context)
    };
}

impl SExp {
    /// Evaluate an S-Expression in a context.
    ///
    /// The context will retain any definitions bound during evaluation
    /// (e.g. `define`, `set!`).
    ///
    /// # Examples
    /// ```
    /// use parsley::prelude::*;
    /// let result = sexp![SExp::sym("eq?"), 0, 1].eval(&mut Context::base());
    /// assert_eq!(result.unwrap(), SExp::from(false));
    /// ```
    /// ```
    /// use parsley::prelude::*;
    /// let exp1 = sexp![SExp::sym("define"), SExp::sym("x"), 10];
    /// let exp2 = SExp::sym("x");
    ///
    /// let mut ctx = Context::base();
    ///
    /// exp1.eval(&mut ctx);
    /// assert_eq!(exp2.eval(&mut ctx).unwrap(), SExp::from(10));
    /// ```
    pub fn eval(self, ctx: &mut Context) -> Result {
        match self {
            Null => Err(Error::NullList),
            Atom(Primitive::Symbol(sym)) => match ctx.get(&sym) {
                None | Some(Atom(Primitive::Undefined)) => Err(Error::UndefinedSymbol { sym }),
                Some(exp) => Ok(exp),
            },
            Atom(_) | Vector(_) => Ok(self),
            Pair { head, tail } => {
                let proc = head.eval(ctx)?;
                if let Atom(Primitive::Procedure { f: Ctx(_), .. }) = proc {
                    *tail
                } else {
                    tail.into_iter().map(|e| e.eval(ctx)).collect::<Result>()?
                }
                .cons(proc)
                .apply(ctx)
            }
        }
    }

    fn apply(self, ctx: &mut Context) -> Result {
        match self {
            Null | Atom(_) | Vector(_) => Ok(self),
            Pair { head, tail } => match *head {
                Atom(Primitive::Procedure { f, env, .. }) => {
                    ctx.overlay_env(env);
                    let result = match f {
                        Basic(p) => p(*tail),
                        Ctx(p) => p(*tail, ctx),
                    };
                    ctx.overlay_env(None);
                    result
                }
                Atom(Primitive::Symbol(sym)) => Err(Error::NotAProcedure {
                    exp: sym.to_string(),
                }),
                Pair {
                    head: proc,
                    tail: tail2,
                } => tail2.cons(proc.eval(ctx)?).eval(ctx),
                _ => Ok(tail.cons(*head)),
            },
        }
    }
}