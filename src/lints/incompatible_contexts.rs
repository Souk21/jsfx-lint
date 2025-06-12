use crate::context::{ContextDemander, MaybeContext};
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            match &fun_def.context {
                MaybeContext::Some(_) | MaybeContext::Unknown | MaybeContext::None => {}
                MaybeContext::HasIncompatibleDemanders(demanders) => {
                    let mut string = String::new();
                    for (i, demander) in demanders.iter().enumerate() {
                        let comma = if i == demanders.len() - 1 {
                            " and "
                        } else if i > 0 {
                            ", "
                        } else {
                            " "
                        };
                        match demander {
                            ContextDemander::FunctionCall { fun_name, context } => {
                                string = format!(
                                    "{string}{comma}{fun_name}() only compatible in {context}"
                                );
                            }
                            ContextDemander::Variable(builtin_var) => {
                                string = format!(
                                    "{string}{comma}{} only compatible in {}",
                                    builtin_var.name,
                                    builtin_var.context.as_ref().expect("Builtin variable that is a ContextDemander should have a context")
                                );
                            }
                        }
                    }
                    let location = fun_def
                        .location
                        .as_ref()
                        .expect("Function definition has no location");
                    issues.add(
                        IssueKind::IncompatibleContexts,
                        location,
                        format!(
                            "Function {}() has incompatible contexts:{}",
                            fun_def.name, string
                        ),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::IssueKind;
    use crate::file::File;
    use indoc::indoc;

    #[test]
    fn incompatible_functions() {
        let source = indoc! {"
            @init
            function foo() (
                // only valid in @sample
                spl(0);
                // only valid in @gfx
                set_host_numchan(0);
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::IncompatibleContexts));
    }

    #[test]
    fn incompatible_builtin() {
        let source = indoc! {"
            @init
            function foo() (
                // only valid in @sample
                _ = spl0;
                // only valid in @gfx
                _ = mouse_cap;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::IncompatibleContexts));
    }

    #[test]
    fn incompatible_mixed() {
        let source = indoc! {"
            @init
            function foo() (
                // only valid in @sample
                spl(0);
                // only valid in @gfx
                _ = mouse_cap;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::IncompatibleContexts));
    }

    #[test]
    fn compatible() {
        let source = indoc! {"
            @init
            function foo() (
                // both valid in anything but @init
                _ = beat_position;
                _ = tempo;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::IncompatibleContexts));
    }

    #[test]
    fn nested() {
        let source = indoc! {"
            @init
            function foo() (
                // only valid in @sample
                spl(0);
            );
            function bar() (
                // only valid in @gfx
                set_host_numchan(0);
            );
            function baz() (
                foo();
                bar();
            );
       "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::IncompatibleContexts));
    }
}
