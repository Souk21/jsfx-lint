use crate::functions::Fun;
use crate::location::Location;
use crate::{IssueKind, Program, issue::IssueTracker};
use std::rc::Rc;

/// Report wrong number of arguments
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            let Some(fun) = &fun_call.fun else {
                // Unknown function
                continue;
            };
            let param_count = fun_call.params.len();
            warn_param_count(fun, param_count, issues, &fun_call.location);
        }
    }
}

fn warn_param_count(
    found: &Rc<Fun>,
    param_count: usize,
    issues: &mut IssueTracker,
    location: &Location,
) {
    let (min, max) = crate::functions::get_args_min_max(&found.args);
    if param_count < min {
        // Can't pass 0 implicitly to a ref arg (that expects a namespace)
        let first_arg_is_ref = found.args.first().is_some_and(|arg| arg.is_ref);
        if !first_arg_is_ref && param_count == 0 && min == 1 {
            let arg_name = found.args.first().map_or("", |arg| arg.name.as_str());
            issues.add(
                IssueKind::ImplicitlyPassingZero,
                location,
                format!(
                    "Implicitly passing 0 as first arg to {}(). {}() expects 1 param `{arg_name}`",
                    found.name, found.name
                ),
            );
        } else {
            issues.add(
                IssueKind::ParamCount,
                location,
                format!(
                    "Not enough params for {} (expected {min} got {param_count})",
                    found.name
                ),
            );
        }
    } else if param_count > max {
        if param_count == 1 && max == 0 {
            issues.add(
                IssueKind::DiscardedParam,
                location,
                format!("{}() param is discarded", found.name),
            );
        } else {
            issues.add(
                IssueKind::ParamCount,
                location,
                format!(
                    "Too many params for {} (expected {max} got {param_count})",
                    found.name
                ),
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
    fn implicitly_passing_zero() {
        let source = indoc! {"
            @init
            function foo(bar) ( 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ImplicitlyPassingZero));
    }

    #[test]
    fn implicitly_passing_zero_namespace() {
        // This should trigger ParamCount and not ImplicitlyPassingZero, as you can't explicitly pass 0 to ref args
        let source = indoc! {"
            @init
            function foo(a*) ( 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ParamCount));
    }

    #[test]
    fn implicitly_discarding() {
        let source = indoc! {"
            @init
            function foo() ( 0; );
            foo(1);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DiscardedParam));
    }
}
