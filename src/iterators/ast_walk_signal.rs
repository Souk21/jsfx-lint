use uuid::Uuid;

use crate::ast::Ast;
use crate::located_ast::LocatedAst;

/// An iterator that walks through AST nodes in the order they would be visited by an interpreter.
/// Each iteration yields a `WalkSignal` marking entry or exit from a node.
pub struct AstWalkSignal<'a> {
    stack: Vec<WalkSignal<'a>>,
}

/// Describes a traversal event for an AST node.
/// `Enter` signals the beginning of a node visit.
/// `Exit` signals the end of a node visit.
pub enum WalkSignal<'a> {
    Exit(&'a LocatedAst),
    Enter(&'a LocatedAst),
}

impl WalkSignal<'_> {
    pub fn is_entering_function(&self) -> bool {
        matches!(
            self,
            WalkSignal::Enter(LocatedAst {
                ast: Ast::Fun { .. },
                ..
            })
        )
    }
    pub fn is_exiting_function(&self) -> Option<(&LocatedAst, &LocatedAst)> {
        match self {
            WalkSignal::Exit(
                ast_loc @ LocatedAst {
                    ast: Ast::Fun { body, .. },
                    ..
                },
            ) => Some((ast_loc, body)),
            _ => None,
        }
    }

    pub fn is_entering_fun_call(&self) -> Option<(&LocatedAst, &Option<Vec<LocatedAst>>, &Uuid)> {
        match self {
            WalkSignal::Enter(LocatedAst {
                ast: Ast::FunCall {
                    name, params, uuid, ..
                },
                ..
            }) => Some((name, params, uuid)),
            _ => None,
        }
    }
}

impl<'a> AstWalkSignal<'a> {
    pub fn new(root: &'a LocatedAst) -> Self {
        AstWalkSignal {
            stack: vec![WalkSignal::Enter(root)],
        }
    }
    fn enter_and_exit(&mut self, ast: &'a LocatedAst) {
        self.stack.push(WalkSignal::Exit(ast));
        self.stack.push(WalkSignal::Enter(ast));
    }
}

impl<'a> Iterator for AstWalkSignal<'a> {
    type Item = WalkSignal<'a>;
    #[allow(clippy::too_many_lines)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.stack.is_empty() {
            return None;
        }
        let cur = self.stack.pop()?;
        if let WalkSignal::Enter(cur_ast) = cur {
            match &cur_ast.ast {
                Ast::Program(e) => self.enter_and_exit(e),
                Ast::Add { lhs, rhs }
                | Ast::Sub { lhs, rhs }
                | Ast::Mul { lhs, rhs }
                | Ast::Div { lhs, rhs }
                | Ast::Pow { lhs, rhs }
                | Ast::LogicalAndOr { lhs, rhs, .. }
                | Ast::Cmp { lhs, rhs, .. }
                | Ast::AndOr { lhs, rhs, .. }
                | Ast::ModShift { lhs, rhs, .. }
                | Ast::Assignment { lhs, rhs, .. } => {
                    self.enter_and_exit(rhs);
                    self.enter_and_exit(lhs);
                }
                Ast::Compound { expressions, .. } => {
                    for e in expressions.iter().rev() {
                        self.enter_and_exit(e);
                    }
                }
                Ast::String { value: _, next } => {
                    if let Some(next) = &next {
                        self.enter_and_exit(next);
                    }
                }
                Ast::FunMod {
                    kind: _,
                    args,
                    commas,
                } => {
                    if let Some(commas) = &commas {
                        self.enter_and_exit(commas);
                    }
                    if let Some(args) = &args {
                        for a in args.iter().rev() {
                            self.enter_and_exit(a);
                        }
                    }
                }
                Ast::Arg { identifier, .. } => {
                    self.enter_and_exit(identifier);
                }
                Ast::Fun {
                    identifier,
                    modifiers,
                    body,
                    args,
                    ..
                } => {
                    self.enter_and_exit(body);
                    if let Some(modifiers) = &modifiers {
                        for m in modifiers.iter().rev() {
                            self.enter_and_exit(m);
                        }
                    }
                    if let Some(args) = &args {
                        for a in args.iter().rev() {
                            self.enter_and_exit(a);
                        }
                    }
                    self.enter_and_exit(identifier);
                }
                Ast::If { condition, yes, no } => {
                    if let Some(no) = &no {
                        self.enter_and_exit(no);
                    }
                    if let Some(yes) = &yes {
                        self.enter_and_exit(yes);
                    }
                    self.enter_and_exit(condition);
                }
                Ast::Unary {
                    operator: _,
                    operand,
                } => {
                    self.enter_and_exit(operand);
                }
                Ast::FunCall { name, params, .. } => {
                    if let Some(params) = &params {
                        for p in params.iter().rev() {
                            self.enter_and_exit(p);
                        }
                    }
                    self.enter_and_exit(name);
                }
                Ast::MemoryAccess { rvalue, index } => {
                    if let Some(index) = &index {
                        self.enter_and_exit(index);
                    }
                    self.enter_and_exit(rvalue);
                }
                Ast::Loop { count, body } => {
                    self.enter_and_exit(body);
                    self.enter_and_exit(count);
                }
                Ast::While { condition, body } => {
                    if let Some(body) = &body {
                        self.enter_and_exit(body);
                    }
                    if let Some(condition) = &condition {
                        self.enter_and_exit(condition);
                    }
                }
                Ast::Identifier { .. }
                | Ast::StringIdentifier { .. }
                | Ast::Unnecessary { .. }
                | Ast::Number(_)
                | Ast::Void
                | Ast::CharLit(_) => (),
            }
        }
        Some(cur)
    }
}
