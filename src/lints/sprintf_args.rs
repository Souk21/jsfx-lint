use crate::functions::{FunCall, ParamKind};
use crate::{IssueKind, Program, issue::IssueTracker};
use regex::Regex;
use std::rc::Rc;

/// Report mistakes with sprintf arguments
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            warn_sprintf_args(fun_call, issues);
        }
    }
}

fn warn_sprintf_args(fun_call: &Rc<FunCall>, issues: &mut IssueTracker) {
    let Some(fun) = &fun_call.fun else {
        // Unknown function
        return;
    };
    if fun.name.as_str() != "sprintf" {
        return;
    }
    let Some(formats) = fun_call.params.get(1) else {
        return;
    };
    let reg = Regex::new(r"(%%[^{% ]+)|(%[^{% ]+)").unwrap();
    for format in formats {
        if let ParamKind::StringValue { value } = &format.kind {
            // Count "%"
            // Only count the 2nd capturing group, the first is there to make sure "%%" is not matched
            // as there are no back-ref in rust regex
            let matches = reg
                .captures_iter(value)
                .filter(|c| c.get(1).is_none())
                .count();
            if fun_call.params.len() != matches + 2 {
                issues.add(
                    IssueKind::SprintfParams,
                    &fun_call.location,
                    format!(
                        "sprintf format string has {matches} parameters but was called with {}",
                        fun_call.params.len() - 2
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn sprintf_too_many_args() {
        let source = indoc! {r#"
            @init
            sprintf(#, "%d%X", 1, 2, 3);
            "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SprintfParams));
    }
    #[test]
    fn sprintf_too_few_args() {
        let source = indoc! {r#"
            @init
            sprintf(#, "%x%c", 1);
            "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SprintfParams));
    }

    #[test]
    fn sprintf_exact_args_no_warning() {
        let source = indoc! {r#"
            @init
            sprintf(#, "%x%X", 1, 2);
            "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SprintfParams));
    }

    #[test]
    fn sprintf_mixed_args_no_warning() {
        let source = indoc! {r#"
            @init
            bar = 2;
            sprintf(#foo, "%d.%{bar}d", 10);
            "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SprintfParams));
    }

    #[test]
    fn sprintf_mixed_args_and_too_many() {
        let source = indoc! {r#"
            @init
            bar = 2;
            sprintf(#foo, "%d.%{bar}d", 10, 20);
            "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SprintfParams));
    }

    #[test]
    fn double_percent_sign_not_counted() {
        let source = indoc! {r#"
            @init
            sprintf(#foo, "%d%%", 10);
            "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SprintfParams));
    }
}
