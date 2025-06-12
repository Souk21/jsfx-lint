use crate::access::var_kind::VarKind;
use crate::functions::ModifierKind;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    function_casing(program, issues);
    variable_casing(program, issues);
    argument_casing(program, issues);
    ref_arg_casing(program, issues);
    local_global_arg_casing(program, issues);
    instance_arg_casing(program, issues);
}

fn instance_arg_casing(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (kind, modifiers) in &fun_def.modifiers {
                if !matches!(kind, ModifierKind::Instance) {
                    continue;
                }
                for modifier in modifiers {
                    for arg in &modifier.args {
                        for access in &fun_def.scope.accesses {
                            let VarKind::Instance { .. } = &access.var_kind else {
                                continue;
                            };
                            let without_suffix = access.info.accessed_as.to_lower()
                                == arg.name.to_lower()
                                && access.info.accessed_as != arg.name;
                            let with_suffix = access
                                .info
                                .accessed_as
                                .strip_prefix_case(&format!("{}.", arg.name.as_str()))
                                .is_some()
                                && &access.info.accessed_as[..arg.name.len()] != arg.name.as_str();
                            if without_suffix || with_suffix {
                                issues.add(
                                    IssueKind::InconsistentCasing,
                                    &access.info.location,
                                    format!(
                                        "Casing is inconsistent, instance() argument {} was accessed as {}",
                                        arg.name, access.info.accessed_as
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn local_global_arg_casing(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for access in &fun_def.scope.accesses {
                if !matches!(access.var_kind, VarKind::Local | VarKind::Global { .. }) {
                    continue;
                }
                for (kind, modifiers) in &fun_def.modifiers {
                    if !matches!(kind, ModifierKind::Local | ModifierKind::Global) {
                        continue;
                    }

                    for modifier in modifiers {
                        for arg in &modifier.args {
                            if access.info.accessed_as.to_lower() == arg.name.to_lower()
                                && access.info.accessed_as != arg.name
                            {
                                issues.add(
                                    IssueKind::InconsistentCasing,
                                    &access.info.location,
                                    format!(
                                        "Casing is inconsistent, modifier argument `{}` was accessed as `{}`",
                                        arg.name, access.info.accessed_as
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ref_arg_casing(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (arg_index, arg) in fun_def.args.iter().enumerate() {
                if !arg.is_ref {
                    continue;
                }
                for access in &fun_def.scope.accesses {
                    let VarKind::RefArg {
                        suffix,
                        arg_index: idx,
                    } = &access.var_kind
                    else {
                        continue;
                    };
                    if arg_index != *idx {
                        continue;
                    }
                    let arg_name = if suffix.as_str().is_empty() {
                        arg.name.as_str()
                    } else {
                        &format!("{}{}", arg.name, suffix)
                    };
                    let aa = access.info.accessed_as.as_str();
                    if aa != arg_name {
                        issues.add(
                            IssueKind::InconsistentCasing,
                            &access.info.location,
                            format!(
                                "Casing is inconsistent, ref argument {} was accessed as {}",
                                arg.name, access.info.accessed_as
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn argument_casing(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (arg_index, arg) in fun_def.args.iter().enumerate() {
                for access in &fun_def.scope.accesses {
                    if arg.is_ref {
                        continue;
                    }
                    if !matches!(access.var_kind, VarKind::Arg { arg_index: idx } if idx == arg_index)
                    {
                        continue;
                    }
                    if access.info.accessed_as != arg.name {
                        issues.add(
                            IssueKind::InconsistentCasing,
                            &access.info.location,
                            format!(
                                "Casing is inconsistent, argument {} was accessed as {}",
                                arg.name, access.info.accessed_as
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn variable_casing(program: &Program, issues: &mut IssueTracker) {
    for variable in program.scope.variables.values() {
        for access in &variable.accesses {
            if access.origin.get_uuid().is_some() {
                // Ignore accesses as ref/this/instance
                continue;
            }
            if access.info.accessed_as != variable.name {
                issues.add(
                    IssueKind::InconsistentCasing,
                    &access.info.location,
                    format!(
                        "Casing is inconsistent: `{}` was originally declared as `{}`",
                        &access.info.accessed_as, variable.name
                    ),
                );
            }
        }
    }
}

fn function_casing(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            let Some(fun) = &fun_call.fun else {
                continue;
            };
            let called_name = fun_call.fun_name();
            if called_name != fun.name.as_str() {
                issues.add(
                    IssueKind::InconsistentCasing,
                    &fun_call.location,
                    format!(
                        "Casing is inconsistent: function `{}()` was invoked as `{}()`",
                        fun.name, called_name
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
    fn inconsistent_fn_casing() {
        let source = indoc! {"
            @init
            function foo() ( bar = 0; );
            Foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_var_casing() {
        let source = indoc! {"
            @init
            foo = 0;
            bar = Foo;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_arg_casing() {
        let source = indoc! {"
            @init
            function foo(Bar) ( bar = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_ref_arg_casing() {
        let source = indoc! {"
            @init
            function foo(bar*) ( Bar = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_ref_arg_with_suffix_casing() {
        let source = indoc! {"
            @init
            function foo(bar*) ( Bar.baz = 0; );
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_modifier_arg_casing() {
        let source = indoc! {"
            @init
            function foo() local(bar) ( Bar = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_modifier_arg_with_suffix_casing() {
        let source = indoc! {"
            @init
            function foo() instance(bar) ( Bar.baz = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn inconsistent_instance_arg_casing() {
        let source = indoc! {"
            @init
            function foo() instance(bar) ( Bar = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn consistent_casing() {
        let source = indoc! {"
            @init
            function foo() ( bar = 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn consistent_casing_with_suffix() {
        let source = indoc! {"
            @init
            function foo(bar*) ( bar.baz = 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn consistent_arg_casing() {
        let source = indoc! {"
            @init
            function foo(bar) ( bar = 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn consistent_modifier_arg_casing() {
        let source = indoc! {"
            @init
            function foo() global(bar) ( bar = 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InconsistentCasing));
    }

    #[test]
    fn consistent_instance_casing() {
        let source = indoc! {"
            @init
            function foo() instance(bar) ( bar = 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InconsistentCasing));
    }
    #[test]
    fn lookalike() {
        let source = indoc! {"
            @init
            function foo()
            instance(
              c,
              Cb0
            )
            (
              Cb0 = 1;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InconsistentCasing));
    }
}
