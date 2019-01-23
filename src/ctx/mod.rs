use std::mem;
use std::rc::Rc;

use super::proc::{Func, Proc};
use super::Primitive::{self, Undefined};
use super::SExp::{self, Atom};
use super::{Cont, Env, Error, Result};

mod base;
mod core;
mod math;
mod write;

/// Evaluation context for LISP expressions.
///
/// ## Note
/// `Context::default()` only provides *very* basic utilities. To obtain an
/// evaluation context with useful functions available, use
/// [`Context::base()`](#method.base).
///
/// ## Some implementation details
/// `Context` maintains separate environments for "core" (special forms, etc.),
/// "lang" (basic functions, vectors, and more), and "user" definitions. Most of
/// the provided methods operate on the "user" environment, as the intended use
/// case keeps the other environments immutable once they have been initialized.
pub struct Context {
    core: Env,
    cont: Rc<Cont>,
    /// You can `insert` additional definitions here to make them available
    /// throughout the runtime. These definitions will not go out of scope
    /// automatically, but can be overridden (see [`get`](#method.get) for
    /// semantic details).
    pub lang: Env,
    user: Vec<Env>,
    out: Option<String>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            core: Self::core(),
            cont: Cont::default().as_link(),
            lang: Env::new(),
            user: vec![Env::new()],
            out: None,
        }
    }
}

impl Context {
    /// Add a new, nested scope.
    ///
    /// See [Context::pop](#method.pop) for a usage example.
    pub fn push(&mut self) {
        self.user.push(Env::new());
    }

    /// Remove the most recently added scope.
    ///
    /// If the stack height is 1, all definitions will be cleared, and the
    /// global scope will be replaced with an empty one.
    ///
    /// # Example
    /// ```
    /// use parsley::prelude::*;
    /// let mut ctx = Context::default();
    ///
    /// assert_eq!(ctx.get("x"), None);
    /// ctx.push();
    /// ctx.define("x", SExp::Null);
    /// assert_eq!(ctx.get("x"), Some(SExp::Null));
    /// ctx.pop();
    /// assert_eq!(ctx.get("x"), None);
    /// ```
    pub fn pop(&mut self) {
        self.user.pop();

        if self.user.is_empty() {
            self.push();
        }
    }

    /// Create a new definition in the current scope.
    pub fn define(&mut self, key: &str, value: SExp) {
        let num_frames = self.user.len();
        self.user[num_frames - 1].insert(key.to_string(), value);
    }

    fn get_user(&self, key: &str) -> Option<SExp> {
        self.user
            .iter()
            .rev()
            .find_map(|w| w.get(key))
            .map(Clone::clone)
    }

    /// Get the definition for a symbol in the execution environment.
    ///
    /// Returns `None` if no definition is found.
    ///
    /// # Override semantics
    /// This method searches for a definition in the following order:
    ///
    ///   1. The core language
    ///   2. The current closure overlay (if there is one)
    ///   3. User definitions, starting from the most recent scope and working
    ///      backward to the top-level
    ///   4. [Language-level definitions](#structfield.lang)
    ///
    /// What this means is that definitions populated in the `lang` field can be
    /// overridden inside the runtime (e.g. in a REPL), but special form keywords
    /// cannot. For example, we can `(define null "foo")`, but we cannot
    /// `(set! and or)`.
    ///
    /// # Examples
    /// ```
    /// let ctx = parsley::Context::default(); // only core definitions included
    /// assert!(ctx.get("potato").is_none());
    /// ```
    /// ```
    /// use parsley::prelude::*;
    /// let mut ctx = Context::default();
    ///
    /// ctx.define("x", SExp::from(3));
    /// assert_eq!(ctx.get("x"), Some(SExp::from(3)));
    /// ```
    pub fn get(&self, key: &str) -> Option<SExp> {
        // first check core (reserved keywords)
        if let Some(exp) = self.core.get(key) {
            return Some(exp.clone());
        }

        // then check the overlay
        let env = self.cont.env();
        if let Some(exp) = env.get(key) {
            return Some(exp.clone());
        }

        // then check user definitions (could have overridden library definitions)
        if let Some(exp) = self.get_user(key) {
            return Some(exp);
        }

        // then check the stdlib
        if let Some(exp) = self.lang.get(key) {
            return Some(exp.clone());
        }

        // otherwise fail
        None
    }

    /// Re-bind an existing definition to a new value.
    ///
    /// Returns `Ok` if an existing definition was found and updated. Returns
    /// `Err` if no definition exists.
    ///
    /// # Example
    /// ```
    /// use parsley::prelude::*;
    /// let mut ctx = Context::default();
    ///
    /// assert!(ctx.set("x", SExp::from(false)).is_err());    // Err, because x is not yet defined
    /// ctx.define("x", SExp::from(3));                       // define x
    /// assert_eq!(ctx.get("x"), Some(SExp::from(3)));        // check that its value is 3
    /// assert!(ctx.set("x", SExp::from("potato")).is_ok());  // Ok because x is now defined
    /// assert_eq!(ctx.get("x"), Some(SExp::from("potato"))); // check that its value is now "potato"
    /// ```
    pub fn set(&mut self, key: &str, value: SExp) -> Result {
        for frame in self.user.iter_mut().rev() {
            if frame.contains_key(key) {
                frame.insert(key.to_string(), value);
                return Ok(Atom(Undefined));
            }
        }
        Err(Error::UndefinedSymbol {
            sym: key.to_string(),
        })
    }

    /// Get a snapshot of environment state for a list of symbols.
    pub fn close(&self, vars: Vec<&str>) -> Env {
        let mut out = Env::new();

        for var in vars {
            if let Some(exp) = self.get_user(var) {
                out.insert(var.to_string(), exp);
            }
        }

        out
    }

    /// Push a new partial continuation onto the stack.
    pub fn push_cont(&mut self, new: Option<Rc<Cont>>) {
        if let Some(c) = new {
            self.cont = Cont::new(Some(self.cont.clone()), c.env()).as_link();
        }
    }

    /// Pop the most recent partial continuation off of the stack.
    pub fn pop_cont(&mut self) {
        let parent = if let Some(parent) = self.cont.parent() {
            parent
        } else {
            Cont::default().as_link()
        };
        mem::replace(&mut self.cont, parent);
    }

    /// Run a code snippet in an existing `Context`.
    ///
    /// # Example
    /// ```
    /// use parsley::prelude::*;
    /// let mut ctx = Context::base();
    ///
    /// assert!(ctx.run("x").is_err());
    /// assert!(ctx.run("(define x 6)").is_ok());
    /// assert_eq!(ctx.run("x").unwrap(), SExp::from(6));
    /// ```
    pub fn run(&mut self, expr: &str) -> Result {
        self.eval(expr.parse::<SExp>()?)
    }

    /// Evaluate an S-Expression in a context.
    ///
    /// The context will retain any definitions bound during evaluation
    /// (e.g. `define`, `set!`).
    ///
    /// # Examples
    /// ```
    /// use parsley::prelude::*;
    /// let result = Context::base().eval(
    ///     sexp![SExp::sym("eq?"), 0, 1]
    /// );
    /// assert_eq!(result.unwrap(), SExp::from(false));
    /// ```
    /// ```
    /// use parsley::prelude::*;
    /// let mut ctx = Context::base();
    ///
    /// let exp1 = sexp![SExp::sym("define"), SExp::sym("x"), 10];
    /// let exp2 = SExp::sym("x");
    ///
    /// ctx.eval(exp1);
    /// assert_eq!(ctx.eval(exp2).unwrap(), SExp::from(10));
    /// ```
    pub fn eval(&mut self, expr: SExp) -> Result {
        use SExp::{Atom, Null, Pair};

        match expr {
            Null => Err(Error::NullList),
            Atom(Primitive::Symbol(sym)) => match self.get(&sym) {
                None | Some(Atom(Primitive::Undefined)) => Err(Error::UndefinedSymbol { sym }),
                Some(exp) => Ok(exp),
            },
            Atom(_) => Ok(expr),
            Pair { head, tail } => {
                let proc = self.eval(*head)?;
                let applic = if let Atom(Primitive::Procedure(Proc {
                    func: Func::Ctx(_), ..
                })) = proc
                {
                    *tail
                } else {
                    tail.into_iter().map(|e| self.eval(e)).collect::<Result>()?
                }
                .cons(proc);
                self.apply(applic)
            }
        }
    }

    fn apply(&mut self, expr: SExp) -> Result {
        use Func::{Ctx, Pure};
        use SExp::{Atom, Null, Pair};

        match expr {
            Null | Atom(_) => Ok(expr),
            Pair { head, tail } => match *head {
                Atom(Primitive::Procedure(proc)) => {
                    proc.check_arity(tail.len())?;
                    self.push_cont(proc.cont);
                    let result = match proc.func {
                        Pure(p) => p(*tail),
                        Ctx(p) => p(self, *tail),
                    };
                    self.pop_cont();
                    result
                }
                Atom(Primitive::Symbol(sym)) => Err(Error::NotAProcedure {
                    exp: sym.to_string(),
                }),
                Pair {
                    head: proc,
                    tail: tail2,
                } => {
                    let the_proc = self.eval(*proc)?;
                    self.eval(tail2.cons(the_proc))
                }
                _ => Ok(tail.cons(*head)),
            },
        }
    }
}
