use crate::{IssueKind, Program, issue::IssueTracker};

/// Report unknown functions
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            if fun_call.fun.is_some() {
                continue;
            }
            issues.add(
                IssueKind::UnknownFunction,
                &fun_call.location,
                format!("Unknown function {}()", fun_call.accessed_as),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn unknown_function() {
        let source = indoc! {"
            @init
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnknownFunction));
    }
}
