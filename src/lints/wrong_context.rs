use crate::context::MaybeContext;
use crate::functions::{Depth, Fun};
use crate::location::Location;
use crate::{IssueKind, Program, issue::IssueTracker, variables::IsBuiltin};
use std::rc::Rc;

/// Report function/variable used in wrong context
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    // Report function called in wrong context
    // only for top-level calls (outside any fn)
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            if !matches!(fun_call.depth, Depth::TopLevel) {
                continue;
            }
            let Some(fun) = &fun_call.fun else {
                continue;
            };
            warn_fn_call_wrong_context(fun, issues, &fun_call.location, section.kind);
        }
    }

    // Report variable used in wrong context
    for (key, variable) in &program.scope.variables {
        let IsBuiltin::BuiltIn(builtin) = program.scope.is_builtin(key, &program.metas) else {
            continue;
        };
        let Some(context) = &builtin.context else {
            continue;
        };
        for access in &variable.accesses {
            let section_kind = access.section;
            let Some(section) = program.sections.get(section_kind) else {
                continue;
            };
            if !context.is_compatible_in_section(section.kind) {
                issues.add(
                    IssueKind::WrongContext,
                    &access.info.location,
                    format!(
                        "`{key}` is used in the wrong context (used in @{}, only valid in {context})",
                        section.kind,
                    ),
                );
            }
        }
    }
}

fn warn_fn_call_wrong_context(
    fun: &Rc<Fun>,
    issues: &mut IssueTracker,
    location: &Location,
    section_kind: &str,
) {
    let MaybeContext::Some(context) = &fun.context else {
        return;
    };
    if !context.is_compatible_in_section(section_kind) {
        issues.add(
            IssueKind::WrongContext,
            location,
            format!(
                "{}() called in @{} but only allowed in {}",
                fun.name, section_kind, context
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::IssueKind;
    use crate::file::File;
    use indoc::indoc;

    #[test]
    fn fun_call_in_fun_call() {
        let source = indoc! {"
            @init
            function foo() (
                // Only valid in @sample
                spl(0);
            );
            @gfx
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongContext));
    }

    #[test]
    fn builtin() {
        let source = indoc! {"
            @init
            _ = spl0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongContext));
    }

    #[test]
    fn builtin_fn() {
        let source = indoc! {"
            @init
            _ = spl();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongContext));
    }

    #[test]
    fn builtin_in_fun() {
        let source = indoc! {"
            @init
            function foo() (
                _ = spl1;
            );
            @gfx
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongContext));
    }
}
