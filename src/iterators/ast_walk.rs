use crate::iterators::ast_walk_signal::{AstWalkSignal, WalkSignal};
use crate::located_ast::LocatedAst;

/// Iterator that yields AST nodes in the order an interpreter would visit them
/// Nodes are yielded as `&LocatedAst`
pub struct AstWalk<'a> {
    it: AstWalkSignal<'a>,
}

impl<'a> AstWalk<'a> {
    pub fn new(root: &'a LocatedAst) -> Self {
        Self {
            it: AstWalkSignal::new(root),
        }
    }
}

impl<'a> Iterator for AstWalk<'a> {
    type Item = &'a LocatedAst;
    fn next(&mut self) -> Option<Self::Item> {
        for sig_ast in &mut self.it {
            if let WalkSignal::Enter(ast) = sig_ast {
                return Some(ast);
            }
        }
        None
    }
}
