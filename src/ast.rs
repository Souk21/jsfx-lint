use crate::{
    functions::ModifierKind,
    located_ast::LocatedAst,
    location::Location,
    operators::{
        AndOrOperator, AssignmentOperator, CmpOperator, LogicalAndOrOperator, ModShiftOperator,
        UnaryOperator,
    },
    rcsubstring::RcSubString,
};
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Debug, Eq)]
pub enum Ast {
    Program(Box<LocatedAst>),
    Fun {
        identifier: Box<LocatedAst>,
        modifiers: Option<Vec<LocatedAst>>,
        body: Box<LocatedAst>,
        args: Option<Vec<LocatedAst>>,
        commas: Option<Box<LocatedAst>>,
        parens: bool,
        uuid: Uuid,
    },
    FunMod {
        kind: ModifierKind,
        args: Option<Vec<LocatedAst>>,
        commas: Option<Box<LocatedAst>>,
    },
    Identifier {
        value: RcSubString,
        is_ref: bool,
    },
    StringIdentifier {
        /// The identifier including the leading '#'
        value: RcSubString,
        is_ref: bool,
    },
    Arg {
        identifier: Box<LocatedAst>,
        leading_commas: Option<(usize, Location)>,
        trailing_comma: Option<(usize, Location)>,
    },
    Unnecessary {
        lex: String,
    },
    If {
        condition: Box<LocatedAst>,
        yes: Option<Box<LocatedAst>>,
        no: Option<Box<LocatedAst>>,
    },
    LogicalAndOr {
        operator: LogicalAndOrOperator,
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    ModShift {
        operator: ModShiftOperator,
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Cmp {
        operator: CmpOperator,
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    AndOr {
        operator: AndOrOperator,
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Add {
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Sub {
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Mul {
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Div {
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Pow {
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<LocatedAst>,
    },
    Number(RcSubString),
    Void,
    Assignment {
        operator: AssignmentOperator,
        lhs: Box<LocatedAst>,
        rhs: Box<LocatedAst>,
    },
    String {
        value: RcSubString,
        next: Option<Box<LocatedAst>>,
    },
    Compound {
        expressions: Vec<LocatedAst>,
        extra_semicolon: Option<(usize, Location)>,
    },
    While {
        condition: Option<Box<LocatedAst>>,
        body: Option<Box<LocatedAst>>,
    },
    FunCall {
        name: Box<LocatedAst>,
        params: Option<Vec<LocatedAst>>,
        uuid: Uuid,
    },
    MemoryAccess {
        rvalue: Box<LocatedAst>,
        index: Option<Box<LocatedAst>>,
    },
    Loop {
        count: Box<LocatedAst>,
        body: Box<LocatedAst>,
    },
    CharLit(RcSubString),
}

impl std::hash::Hash for Ast {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Ast {
    pub const fn identifier(&self) -> Option<&RcSubString> {
        if let Self::Identifier { value, .. } | Self::StringIdentifier { value, .. } = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn get_entire_string(&self) -> String {
        let Self::String { value, next } = &self else {
            panic!("Expected AST::String");
        };
        let mut next = next;
        let mut ret = value.to_string();
        while let Some(ast_loc) = next {
            if let Self::String {
                value,
                next: next_str,
            } = &ast_loc.ast
            {
                ret += value;
                next = next_str;
            } else {
                break;
            }
        }
        ret
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Program(_) => write!(f, "Program"),
            Self::Fun { identifier, .. } => write!(
                f,
                "Fn {}",
                identifier.ast.identifier().map_or("", RcSubString::as_str)
            ),
            Self::FunMod { kind, .. } => write!(f, "FnMod {kind}"),
            Self::Identifier { value, is_ref } => {
                write!(f, "Identifier {value}{}", if *is_ref { "*" } else { "" })
            }
            Self::StringIdentifier { value, is_ref } => write!(
                f,
                "StringIdentifier {value}{}",
                if *is_ref { "*" } else { "" }
            ),
            Self::Arg { identifier, .. } => write!(
                f,
                "Arg {}",
                identifier.ast.identifier().map_or("", RcSubString::as_str)
            ),
            Self::Unnecessary { .. } => write!(f, "Unnecessary"),
            Self::If { .. } => write!(f, "If"),
            Self::LogicalAndOr { .. } => write!(f, "LogicalAndOr"),
            Self::ModShift { .. } => write!(f, "ModShift"),
            Self::Cmp { .. } => write!(f, "Cmp"),
            Self::AndOr { .. } => write!(f, "AndOr"),
            Self::Add { .. } => write!(f, "Add"),
            Self::Sub { .. } => write!(f, "Sub"),
            Self::Mul { .. } => write!(f, "Mul"),
            Self::Div { .. } => write!(f, "Div"),
            Self::Pow { .. } => write!(f, "Pow"),
            Self::Unary { .. } => write!(f, "Unary"),
            Self::Number(_) => write!(f, "Number"),
            Self::Void => write!(f, "Void"),
            Self::Assignment { .. } => write!(f, "Assignment"),
            Self::String { .. } => write!(f, "String"),
            Self::Compound { .. } => write!(f, "Compound"),
            Self::While { .. } => write!(f, "While"),
            Self::FunCall { .. } => write!(f, "FnCall"),
            Self::MemoryAccess { .. } => write!(f, "MemoryAccess"),
            Self::Loop { .. } => write!(f, "Loop"),
            Self::CharLit(_) => write!(f, "CharLit"),
        }
    }
}

impl PartialEq for Ast {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
