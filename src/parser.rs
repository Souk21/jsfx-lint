use crate::{file::File, location::Location, rcsubstring::RcSubString};
use std::error::Error;
use std::rc::Rc;

#[derive(Debug)]
pub enum ParseError {
    ExpectedWhile(Location),
    ExpectedFunModifier(Location),
    ExpectedFunction(Location),
    /// Third-party lexer error
    DefaultLexeme(lrlex::DefaultLexeme),
}

pub struct ParserParam {
    pub file: Rc<File>,
    pub section_line_pos: usize,
    pub section_kind: &'static str,
    pub source: RcSubString,
}

impl From<lrlex::DefaultLexeme> for ParseError {
    fn from(lexeme: lrlex::DefaultLexeme) -> Self {
        Self::DefaultLexeme(lexeme)
    }
}

impl Error for ParseError {}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ExpectedWhile(_) => write!(f, "Expected 'while' keyword"),
            Self::ExpectedFunModifier(_) => {
                write!(
                    f,
                    "Expected 'global', 'globals', 'local', 'static' or 'instance' keyword"
                )
            }
            Self::ExpectedFunction(_) => {
                write!(f, "Expected 'function' keyword")
            }
            Self::DefaultLexeme(lexeme) => write!(f, "Unexpected lexeme: {lexeme}"),
        }
    }
}
