use std::rc::Rc;
use std::str::FromStr;
use std::string::String as CoreString;

use super::super::{utils, Error, Result, SExp};
use super::proc::Procedure::Basic;
use super::Primitive::{self, Boolean, Character, Number, Procedure, String, Symbol};

impl FromStr for Primitive {
    type Err = Error;

    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        match s {
            "#t" => return Ok(Boolean(true)),
            "#f" => return Ok(Boolean(false)),
            _ => (),
        }

        if let Ok(num) = s.parse::<f64>() {
            return Ok(Number(num));
        }

        if s.len() == 3 && s.starts_with("#\\") {
            return Ok(Character(s.chars().nth(2).unwrap()));
        }

        if s.starts_with('"') && s.ends_with('"') {
            match utils::find_closing_delim(s.chars(), '"', '"') {
                Some(idx) if idx + 1 == s.len() => {
                    return Ok(String(s.get(1..idx).unwrap().to_string()));
                }
                _ => (),
            }
        }

        if s.chars().all(utils::is_symbol_char) {
            return Ok(Symbol(s.to_string()));
        }

        Err(Error::Syntax { exp: s.to_string() })
    }
}

impl From<bool> for Primitive {
    fn from(b: bool) -> Self {
        Boolean(b)
    }
}

impl From<i32> for Primitive {
    fn from(n: i32) -> Self {
        Number(f64::from(n))
    }
}

impl From<f32> for Primitive {
    fn from(n: f32) -> Self {
        Number(f64::from(n))
    }
}

impl From<f64> for Primitive {
    fn from(n: f64) -> Self {
        Number(n)
    }
}

impl From<char> for Primitive {
    fn from(c: char) -> Self {
        Character(c)
    }
}

impl From<&str> for Primitive {
    fn from(s: &str) -> Self {
        String(s.to_string())
    }
}

impl From<CoreString> for Primitive {
    fn from(s: CoreString) -> Self {
        String(s)
    }
}

impl<F> From<F> for Primitive
where
    F: Fn(SExp) -> Result + 'static,
{
    fn from(f: F) -> Self {
        Procedure {
            f: Basic(Rc::new(f)),
            name: None,
            env: None,
        }
    }
}
