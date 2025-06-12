use crate::{IssueKind, Program, functions::Arg, issue::IssueTracker, location::Location};

/// Report unreachable functions
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            warn_unreachable_loop_while_fn(
                &fun_def.name,
                &fun_def.args,
                issues,
                fun_def
                    .location
                    .as_ref()
                    .expect("All non-builtin functions should have a location."),
            );
        }
    }
}

fn warn_unreachable_loop_while_fn(
    identifier: &str,
    args_vec: &[Arg],
    issues: &mut IssueTracker,
    location: &Location,
) {
    if identifier.eq_ignore_ascii_case("while") && args_vec.len() <= 1 {
        let arg = args_vec.first().map_or("", |a| a.name.as_str());
        issues.add(
            IssueKind::UnreachableFunction,
            location,
            format!("User defined function while({arg}) is unreachable"),
        );
    }
    if identifier.eq_ignore_ascii_case("loop") && args_vec.len() == 2 {
        let arg_list = format!("{}, {}", args_vec[0].name, args_vec[1].name);
        issues.add(
            IssueKind::UnreachableFunction,
            location,
            format!("User defined function loop({arg_list}) is unreachable"),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn unreachable_while_function() {
        let source = indoc! {"
            @init
            function while() ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnreachableFunction));
    }

    #[test]
    fn unreachable_loop_function() {
        let source = indoc! {"
            @init
            function loop(a, b) ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnreachableFunction));
    }
}
