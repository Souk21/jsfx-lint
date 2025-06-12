use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::rcsubstring::RcSubString;

use crate::variables::BuiltinVar;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum MaybeContext {
    Some(Context),
    HasIncompatibleDemanders(Vec<ContextDemander>),
    Unknown,
    None,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ContextDemander {
    FunctionCall {
        fun_name: RcSubString,
        context: Context,
    },
    Variable(Rc<BuiltinVar>),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Context {
    bit_field: u8,
}

impl Context {
    pub const fn new() -> Self {
        Self {
            bit_field: 1 + 2 + 4 + 8 + 16 + 32,
        }
    }

    pub fn from_vec(contexts: Vec<&str>) -> Self {
        let mut bit_field = 0;
        for context in contexts {
            bit_field |= Self::string_to_bit(context);
        }
        Self { bit_field }
    }

    pub const fn intersect(&self, other: &Self) -> Self {
        Self {
            bit_field: self.bit_field & other.bit_field,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.bit_field == 0
    }

    pub fn is_compatible_with_all(&self) -> bool {
        self.bit_field == Self::default().bit_field
    }

    pub fn is_compatible_in_section(&self, section: &str) -> bool {
        let bit = Self::string_to_bit(section);
        self.bit_field & bit != 0
    }

    fn string_to_bit(context_name: &str) -> u8 {
        match context_name {
            "init" => 1,
            "serialize" => 2,
            "block" => 4,
            "sample" => 8,
            "gfx" => 16,
            "slider" => 32,
            _ => panic!("Unknown context"),
        }
    }

    fn bit_to_string(bit: u8) -> &'static str {
        match bit {
            1 => "init",
            2 => "serialize",
            4 => "block",
            8 => "sample",
            16 => "gfx",
            32 => "slider",
            _ => panic!("Unknown context"),
        }
    }
}

impl Display for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for i in 0..6 {
            if self.bit_field & (1 << i) != 0 {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "@{}", Self::bit_to_string(1 << i))?;
                first = false;
            }
        }
        Ok(())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
