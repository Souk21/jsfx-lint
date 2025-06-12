use crate::ast::Ast;
use crate::iterators::ast_walk::AstWalk;
use crate::located_ast::LocatedAst;
use crate::{IssueKind, Program, access, issue::IssueTracker};
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for chunk in &section.chunks {
            let Some(root) = chunk.ast.as_ref() else {
                // Chunk is not parsed
                continue;
            };
            for ast in AstWalk::new(root) {
                let LocatedAst {
                    ast: Ast::Compound { expressions, .. },
                    ..
                } = ast
                else {
                    continue;
                };
                let skip_last = expressions.len().saturating_sub(1);
                for expression in &expressions[0..skip_last] {
                    match &expression.ast {
                        Ast::Identifier { .. }
                        | Ast::StringIdentifier { .. }
                        | Ast::Number(_)
                        | Ast::String { .. }
                        | Ast::CharLit(_) => {
                            issues.add(
                                IssueKind::UselessExpression,
                                &expression.location,
                                "Useless expression".into(),
                            );
                        }

                        Ast::FunCall { uuid, .. } => {
                            let fun_call = section
                                .uuid_to_fun_calls
                                .get(uuid)
                                .expect("FnCall should exist");
                            let Some(fun) = fun_call.fun.as_ref() else {
                                // Unknown function
                                continue;
                            };
                            if fun.has_side_effects {
                                continue;
                            }
                            issues.add(
                                IssueKind::UselessExpression,
                                &expression.location,
                                "Useless expression".into(),
                            );
                        }

                        Ast::LogicalAndOr { .. }
                        | Ast::ModShift { .. }
                        | Ast::AndOr { .. }
                        | Ast::Add { .. }
                        | Ast::Sub { .. }
                        | Ast::Mul { .. }
                        | Ast::Div { .. }
                        | Ast::Pow { .. }
                        | Ast::Unary { .. }
                        | Ast::MemoryAccess { .. }
                        | Ast::Cmp { .. } => {
                            // Expression has side effects if it has `write` accesses
                            // Or if it calls functions with side effects
                            let mut accesses = Vec::new();
                            access::get_accesses(expression, &mut accesses, section);
                            if accesses
                                .iter()
                                .filter(|access| access.info.is_write())
                                .count()
                                != 0
                            {
                                continue;
                            }
                            let calls_with_side_effects = AstWalk::new(expression).any(|child| {
                                let Ast::FunCall { uuid, .. } = &child.ast else {
                                    return false;
                                };
                                let fun_call = section
                                    .uuid_to_fun_calls
                                    .get(uuid)
                                    .expect("FnCall should exist");
                                fun_call
                                    .fun
                                    .as_ref()
                                    .is_some_and(|fun| fun.has_side_effects)
                            });
                            if calls_with_side_effects {
                                continue;
                            }
                            issues.add(
                                IssueKind::UselessExpression,
                                &expression.location,
                                "Useless expression".into(),
                            );
                        }
                        _ => (),
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
    fn useless_expression() {
        let source = indoc! {"
            @init
            function foo() (
                1;
                3;
             );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn useful_expression() {
        let source = indoc! {"
            @init
            function foo() (
                3;
             );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn nested() {
        let source = indoc! {"
            @init
            function unit(x) (x);
            unit(2 > 1 ? (
                unit(3 % (a; b))
            ));
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessExpression));
    }
    #[test]
    fn side_effect_fun() {
        let source = indoc! {"
            @init
            function side_effect() (
                a = 1;
            );
            side_effect();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn no_side_effect_fun() {
        let source = indoc! {"
            @init
            function no_side_effect() ( 0; );
            no_side_effect();
            _;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessExpression));
    }
    #[test]
    fn fn_call_nested() {
        let source = indoc! {"
            @init
            function side_effect() (
                a = 1;
            );
            0 && side_effect();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }
    #[test]
    fn fn_call_nested2() {
        let source = indoc! {"
            @init
            0 && (a += 1);
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn mem_access() {
        let source = indoc! {"
            @init
            function foo() (
              1000[0] = 1
            );
            foo();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn nested_mem_access() {
        let source = indoc! {"
            @init
            function foo() (
              1000[0] = 1
            );
            function bar() (
                foo();
            );
            bar();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }
    #[test]
    fn ref_arg() {
        let source = indoc! {"
            @init
            function foo(a*) (
              a = 1
            );
            foo(b);
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn builtin_side_effect() {
        let source = indoc! {"
            @init
            function foo() (
              freembuf(1000);
              _;
            );
            foo();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn builtin_pure() {
        let source = indoc! {"
            @init
            function foo() (
              floor(1000);
              _;
            );
            foo();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessExpression));
    }

    #[test]
    fn nested_builtin() {
        let source = indoc! {"
            @init
            function foo() (
              a > b ? ( floor(10) ) : ( ceil() );
              _;
            );
            foo();
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessExpression));
    }
}
