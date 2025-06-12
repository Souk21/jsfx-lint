use crate::ast::Ast;
use crate::iterators::ast_walk::AstWalk;
use crate::located_ast::LocatedAst;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for chunk in &section.chunks {
            let Some(root) = chunk.ast.as_ref() else {
                // Chunk is not parsed
                continue;
            };
            for LocatedAst { ast, .. } in AstWalk::new(root) {
                let Ast::Assignment { lhs, .. } = ast else {
                    continue;
                };
                let rets = lhs.get_return_values();
                for ret in rets {
                    match &ret.ast {
                        Ast::Program(_)
                        | Ast::Fun { .. }
                        | Ast::FunMod { .. }
                        | Ast::Arg { .. }
                        | Ast::If { .. }
                        | Ast::Identifier { .. }
                        | Ast::StringIdentifier { .. }
                        | Ast::Assignment { .. }
                        | Ast::Compound { .. }
                        | Ast::MemoryAccess { .. }
                        | Ast::Unnecessary { .. }
                        | Ast::Void => (),

                        Ast::LogicalAndOr { .. }
                        | Ast::Cmp { .. }
                        | Ast::AndOr { .. }
                        | Ast::Add { .. }
                        | Ast::Sub { .. }
                        | Ast::Mul { .. }
                        | Ast::Div { .. }
                        | Ast::Pow { .. }
                        | Ast::Unary { .. }
                        | Ast::Number(_)
                        | Ast::ModShift { .. }
                        | Ast::String { .. }
                        | Ast::While { .. }
                        | Ast::Loop { .. }
                        | Ast::CharLit(_) => {
                            issues.add(
                                IssueKind::InvalidLhsAssignment,
                                &ret.location,
                                "Invalid LHS operand".into(),
                            );
                        }
                        Ast::FunCall { name, .. } => {
                            let identifier = name
                                .ast
                                .identifier()
                                .expect("FnCall name is not an identifier");
                            // It's allowed to do `spl(0) += spl(1)` or `slider(1) = 2;`
                            if identifier.to_lower() != "spl" && identifier.to_lower() != "slider" {
                                issues.add(
                                    IssueKind::InvalidLhsAssignment,
                                    &ret.location,
                                    "Invalid LHS operand".into(),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn assigning_to_a_fun_call() {
        let source = indoc! {"
            @init
            floor(0) += 0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InvalidLhsAssignment));
    }

    #[test]
    fn assigning_to_a_number() {
        let source = indoc! {"
            @init
            (0) += 0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InvalidLhsAssignment));
    }

    #[test]
    fn assigning_to_spl() {
        let source = indoc! {"
            @sample
            spl(1) = 0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InvalidLhsAssignment));
    }
    #[test]
    fn assigning_to_an_identifier() {
        let source = indoc! {"
            @init
            foo = 0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InvalidLhsAssignment));
    }
}
