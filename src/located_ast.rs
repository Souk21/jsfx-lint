use crate::{ast::Ast, location::Location};
use std::fmt::{Display, Formatter};

#[derive(Debug, Eq)]
pub struct LocatedAst {
    pub ast: Ast,
    pub location: Location,
}

impl PartialEq for LocatedAst {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl std::hash::Hash for LocatedAst {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ast.hash(state);
        self.location.hash(state);
    }
}

impl LocatedAst {
    pub fn get_return_values(&self) -> Vec<&Self> {
        match &self.ast {
            Ast::Fun { body, .. } => body.get_return_values(),
            Ast::Assignment { lhs, .. } => lhs.get_return_values(),

            Ast::Compound { expressions, .. } => expressions
                .last()
                .map_or_else(|| vec![self], |last| last.get_return_values()),

            Ast::If { yes, no, .. } => match (yes, no) {
                (Some(yes), Some(no)) => {
                    let mut ret = yes.get_return_values();
                    ret.append(&mut no.get_return_values());
                    ret
                }
                (Some(yes), None) => yes.get_return_values(),
                (None, Some(no)) => no.get_return_values(),
                (None, None) => panic!("Expected at least one branch"),
            },

            Ast::While { .. }
            | Ast::AndOr { .. }
            | Ast::LogicalAndOr { .. }
            | Ast::ModShift { .. }
            | Ast::Cmp { .. }
            | Ast::Add { .. }
            | Ast::Pow { .. }
            | Ast::Div { .. }
            | Ast::Sub { .. }
            | Ast::Mul { .. }
            | Ast::Loop { .. }
            | Ast::MemoryAccess { .. }
            | Ast::StringIdentifier { .. }
            | Ast::Unary { .. }
            | Ast::FunCall { .. }
            | Ast::CharLit(_)
            | Ast::String { .. }
            | Ast::Number(_)
            | Ast::Identifier { .. } => vec![self],

            Ast::Void
            | Ast::Program(_)
            | Ast::FunMod { .. }
            | Ast::Arg { .. }
            | Ast::Unnecessary { .. } => panic!("Unexpected {self:?}"),
        }
    }
}

impl Display for LocatedAst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ast)
    }
}
