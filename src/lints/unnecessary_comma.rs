use crate::ast::Ast;
use crate::functions::ModifierKind;
use crate::iterators::ast_walk::AstWalk;
use crate::{
    IssueKind, Program, issue::IssueTracker, located_ast::LocatedAst, location::Location,
    section::Section,
};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    // In Fun (only if no arguments, otherwise unnecessary commas go to Fun Arg)
    warn_unnecessary_comma_in_fun(program, issues);
    // In Fun arguments
    warn_unnecessary_comma_in_fun_arg(program, issues);
    // In FnMod (only if no arguments, otherwise unnecessary commas go to FnMod Arg)
    warn_unnecessary_comma_in_modifier(program, issues);
    // In FnMod Args
    warn_unnecessary_comma_in_modifier_arg(program, issues);
}

fn warn_unnecessary_comma_in_fun(program: &Program, issues: &mut IssueTracker) {
    for Section { chunks, .. } in program.sections.values() {
        for chunk in chunks {
            let Some(parsed) = chunk.ast.as_ref() else {
                // Section is not fully parsed, skip it
                continue;
            };
            for LocatedAst { ast, .. } in AstWalk::new(parsed) {
                let Ast::Fun { commas, .. } = ast else {
                    continue;
                };
                let Some(commas) = commas.as_ref() else {
                    continue;
                };
                let Ast::Unnecessary { lex } = &commas.ast else {
                    panic!("Expected Ast::Unnecessary");
                };
                issues.add(
                    IssueKind::UnnecessaryComma,
                    &commas.location,
                    format!("Unnecessary '{lex}'"),
                );
            }
        }
    }
}

fn warn_unnecessary_comma_in_fun_arg(program: &Program, issues: &mut IssueTracker) {
    for Section { chunks, .. } in program.sections.values() {
        for chunk in chunks {
            let Some(parsed) = chunk.ast.as_ref() else {
                // Section is not fully parsed, skip it
                continue;
            };
            for LocatedAst { ast, .. } in AstWalk::new(parsed) {
                let Ast::Fun {
                    args: Some(args),
                    identifier,
                    ..
                } = ast
                else {
                    continue;
                };
                let fun_name = identifier.ast.identifier().expect("Fn without identifier");
                for arg in args {
                    let Ast::Arg {
                        leading_commas,
                        trailing_comma,
                        ..
                    } = &arg.ast
                    else {
                        continue;
                    };
                    warn_commas_in_arg(leading_commas, issues, fun_name, trailing_comma);
                }
            }
        }
    }
}

fn warn_unnecessary_comma_in_modifier(program: &Program, issues: &mut IssueTracker) {
    for Section { chunks, .. } in program.sections.values() {
        for chunk in chunks {
            let Some(parsed) = chunk.ast.as_ref() else {
                // Section is not fully parsed, skip it
                continue;
            };
            for LocatedAst { ast, .. } in AstWalk::new(parsed) {
                let Ast::Fun {
                    modifiers: Some(modifiers),
                    identifier,
                    ..
                } = ast
                else {
                    continue;
                };
                let fun_name = identifier.ast.identifier().expect("Fn without identifier");
                for modifier in modifiers {
                    let Ast::FunMod {
                        commas: Some(commas),
                        kind: mod_type,
                        ..
                    } = &modifier.ast
                    else {
                        continue;
                    };
                    issues.add(
                        IssueKind::UnnecessaryComma,
                        &commas.location,
                        format!("Unnecessary commas in {fun_name}() {mod_type}() arg list"),
                    );
                }
            }
        }
    }
}

fn warn_unnecessary_comma_in_modifier_arg(program: &Program, issues: &mut IssueTracker) {
    for Section { chunks, .. } in program.sections.values() {
        for chunk in chunks {
            let Some(parsed) = chunk.ast.as_ref() else {
                // Section is not fully parsed, skip it
                continue;
            };
            for LocatedAst { ast, .. } in AstWalk::new(parsed) {
                let Ast::Fun {
                    modifiers: Some(modifiers),
                    identifier,
                    ..
                } = ast
                else {
                    continue;
                };
                for modifier in modifiers {
                    let Ast::FunMod {
                        args: Some(args),
                        kind: mod_type,
                        ..
                    } = &modifier.ast
                    else {
                        continue;
                    };
                    let fun_name = identifier.ast.identifier().expect("Fn without identifier");
                    for arg in args {
                        let Ast::Arg {
                            leading_commas: comma_count,
                            trailing_comma,
                            ..
                        } = &arg.ast
                        else {
                            continue;
                        };
                        warn_commas_mod_args(
                            comma_count,
                            issues,
                            fun_name,
                            mod_type,
                            trailing_comma,
                        );
                    }
                }
            }
        }
    }
}

fn warn_commas_mod_args(
    leading_commas: &Option<(usize, Location)>,
    issues: &mut IssueTracker,
    fn_name: &str,
    mod_kind: &ModifierKind,
    trailing_comma: &Option<(usize, Location)>,
) {
    if let Some(leading_commas) = leading_commas {
        if leading_commas.0 > 1 {
            let location = &leading_commas.1;
            let new_location = Location {
                line_col: crate::location::LineCol {
                    start_line: location.line_col.start_line,
                    end_line: location.line_col.end_line,
                    start_column: location.line_col.start_column + 1,
                    end_column: location.line_col.end_column,
                },
                file: location.file.clone(),
                section: location.section,
            };
            let s = if leading_commas.0 > 3 { "s" } else { "" };
            issues.add(
                IssueKind::UnnecessaryComma,
                &new_location,
                format!("Unnecessary comma{s} in {fn_name}() {mod_kind}() arg list"),
            );
        }
    }
    if let Some(trailing_comma) = trailing_comma {
        if trailing_comma.0 > 1 {
            let location = &trailing_comma.1;
            let new_location = Location {
                line_col: crate::location::LineCol {
                    start_line: location.line_col.start_line,
                    end_line: location.line_col.end_line,
                    start_column: location.line_col.start_column + 1,
                    end_column: location.line_col.end_column,
                },
                file: location.file.clone(),
                section: location.section,
            };
            let s = if trailing_comma.0 > 3 { "s" } else { "" };
            issues.add(
                IssueKind::UnnecessaryComma,
                &new_location,
                format!("Unnecessary comma{s} at the end of {fn_name}() {mod_kind}() arg list"),
            );
        }
    }
}

fn warn_commas_in_arg(
    leading_commas: &Option<(usize, Location)>,
    issues: &mut IssueTracker,
    fn_name: &str,
    trailing_comma: &Option<(usize, Location)>,
) {
    if let Some(leading_commas) = leading_commas {
        if leading_commas.0 > 1 {
            let location = &leading_commas.1;
            let new_location = Location {
                line_col: crate::location::LineCol {
                    start_line: location.line_col.start_line,
                    end_line: location.line_col.end_line,
                    start_column: location.line_col.start_column + 1,
                    end_column: location.line_col.end_column,
                },
                file: location.file.clone(),
                section: location.section,
            };
            let s = if leading_commas.0 > 3 { "s" } else { "" };
            issues.add(
                IssueKind::UnnecessaryComma,
                &new_location,
                format!("Unnecessary comma{s} in {fn_name}() arg list"),
            );
        }
    }
    if let Some(trailing_comma) = trailing_comma {
        if trailing_comma.0 > 1 {
            let location = &trailing_comma.1;
            let new_location = Location {
                line_col: crate::location::LineCol {
                    start_line: location.line_col.start_line,
                    end_line: location.line_col.end_line,
                    start_column: location.line_col.start_column + 1,
                    end_column: location.line_col.end_column,
                },
                file: location.file.clone(),
                section: location.section,
            };
            let s = if trailing_comma.0 > 3 { "s" } else { "" };
            issues.add(
                IssueKind::UnnecessaryComma,
                &new_location,
                format!("Unnecessary comma{s} at the end of {fn_name}() arg list"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::IssueKind;
    use crate::file::File;
    use indoc::indoc;

    #[test]
    fn arg_list() {
        let source = indoc! {"
            @init
            function foo(a,,b) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessaryComma));
    }

    #[test]
    fn mod_arg_list() {
        let source = indoc! {"
            @init
            function foo() local(a,,b) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessaryComma));
    }
    #[test]
    fn only_commas_in_args() {
        let source = indoc! {"
            @init
            function foo(,,,) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessaryComma));
    }
    #[test]
    fn only_commas_in_mod_args() {
        let source = indoc! {"
            @init
            function foo() local(,,,) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessaryComma));
    }
    #[test]
    fn after() {
        let source = indoc! {"
            @init
            function foo(a,,) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessaryComma));
    }

    #[test]
    fn after_mod_arg() {
        let source = indoc! {"
            @init
            function foo() local(a,,) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessaryComma));
    }
}
