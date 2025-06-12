use crate::iterators::ast_walk::AstWalk;
use crate::{IssueKind, Program, issue::IssueTracker, located_ast::LocatedAst, section::Section};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for Section { chunks, .. } in program.sections.values() {
        for chunk in chunks {
            let Some(parsed) = chunk.ast.as_ref() else {
                // Section is not fully parsed, skip it
                continue;
            };
            for LocatedAst { ast, location } in AstWalk::new(parsed) {
                let crate::ast::Ast::Fun {
                    identifier, parens, ..
                } = ast
                else {
                    continue;
                };
                let fn_name = identifier.ast.identifier().expect("Fn without identifier");
                if !parens {
                    // Function definition without parenthesis
                    // e.g. function foo local(bar) ( 0; )
                    // no parens here -^
                    issues.add(
                        IssueKind::ImplicitParens,
                        location,
                        format!("Implicit parens for {fn_name} definition"),
                    );
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
    fn implicit_parens() {
        let source = indoc! {"
            @init
            function foo local(bar) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ImplicitParens));
    }
}
