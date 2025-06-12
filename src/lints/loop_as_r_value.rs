use crate::iterators::ast_walk::AstWalk;
use crate::{
    IssueKind, Program, ast::Ast, issue::IssueTracker, located_ast::LocatedAst, section::Section,
};

/// Report loop used as rvalue
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for Section { chunks, .. } in program.sections.values() {
        for chunk in chunks {
            let Some(parsed) = chunk.ast.as_ref() else {
                // Section is not fully parsed, skip it
                continue;
            };
            for ast_loc in AstWalk::new(parsed) {
                if let LocatedAst {
                    ast: Ast::Assignment { rhs, .. },
                    ..
                } = ast_loc
                {
                    let rets_b = rhs.get_return_values();
                    warn_loop_as_rvalue(&rets_b, issues);
                }
            }
        }
    }
}

fn warn_loop_as_rvalue(rets_b: &Vec<&LocatedAst>, issues: &mut IssueTracker) {
    for ret_b in rets_b {
        match ret_b.ast {
            Ast::While { .. } => {
                issues.add(
                    IssueKind::LoopAsRValue,
                    &ret_b.location,
                    "While always return 0".to_string(),
                );
            }
            Ast::Loop { .. } => {
                issues.add(
                    IssueKind::LoopAsRValue,
                    &ret_b.location,
                    "Loop always return 1".to_string(),
                );
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn loop_as_rvalue() {
        let source = indoc! {"
            @init
            _ = loop(1, 2);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::LoopAsRValue));
    }

    #[test]
    fn while_as_rvalue() {
        let source = indoc! {"
            @init
            _ = while(1) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::LoopAsRValue));
    }
}
