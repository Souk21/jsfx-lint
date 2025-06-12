use crate::functions::ModifierKind;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (kind, modifiers) in &fun_def.modifiers {
                if matches!(kind, ModifierKind::Global) {
                    continue;
                }
                for modifier in modifiers {
                    for arg in &modifier.args {
                        if arg.is_ref {
                            issues.add(
                                IssueKind::RefArgInIncompatibleModifier,
                                arg.location.as_ref().expect(
                                    "All non-builtin function's arg should have a location.",
                                ),
                                "Ref arg are only allowed in global()".into(),
                            );
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
    fn ref_arg_in_local() {
        let source = indoc! {"
            @init
            function foo(bar) local(bar*) ( 0 );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::RefArgInIncompatibleModifier));
    }
    #[test]
    fn ref_arg_in_instance() {
        let source = indoc! {"
            @init
            function foo(bar) instance(bar*) ( 0 );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::RefArgInIncompatibleModifier));
    }
}
