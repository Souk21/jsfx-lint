use crate::access::var_kind::VarKind;
use crate::functions::{Depth, Fun, Modifier, ModifierKind};
use crate::section::Section;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (kind, modifiers) in &fun_def.modifiers {
                match kind {
                    ModifierKind::Local => unused_in_local_mods(modifiers, fun_def, issues),
                    ModifierKind::Global => {
                        unused_in_global_mods(modifiers, fun_def, section, issues);
                    }
                    ModifierKind::Instance => {
                        unused_in_instance_mods(modifiers, fun_def, section, issues);
                    }
                }
            }
        }
    }
}

fn unused_in_local_mods(modifiers: &Vec<Modifier>, fun: &Fun, issues: &mut IssueTracker) {
    // Locals should report if they are written but not read, and the opposite.
    for modifier in modifiers {
        for arg in &modifier.args {
            let is_written = fun.scope.accesses.iter().any(|access| {
                matches!(&access.var_kind, VarKind::Local)
                    && access.info.is_write()
                    && access.info.accessed_as.to_lower() == arg.name.to_lower()
            });
            let is_read = fun.scope.accesses.iter().any(|access| {
                matches!(&access.var_kind, VarKind::Local)
                    && access.info.is_read()
                    && access.info.accessed_as.to_lower() == arg.name.to_lower()
            });
            if !is_written && !is_read {
                let location = arg.location.as_ref().expect("Arg should have a Location");
                issues.add(
                    IssueKind::UnusedModifierArg,
                    location,
                    format!(
                        "local variable `{}` is not used in {}()",
                        arg.name, fun.name
                    ),
                );
            } else if !is_written {
                let location = arg.location.as_ref().expect("Arg should have a Location");
                issues.add(
                    IssueKind::UnusedModifierArg,
                    location,
                    format!("local variable {} is always 0 in {}", arg.name, fun.name),
                );
            } else if !is_read {
                let location = arg.location.as_ref().expect("Arg should have a Location");
                issues.add(
                    IssueKind::UnusedModifierArg,
                    location,
                    format!(
                        "local variable {} is never read in {}()",
                        arg.name, fun.name
                    ),
                );
            }
        }
    }
}

fn unused_in_global_mods(
    modifiers: &Vec<Modifier>,
    fun: &Fun,
    section: &Section,
    issues: &mut IssueTracker,
) {
    // Globals are unused if they are not read or written to
    for modifier in modifiers {
        for arg in &modifier.args {
            let is_accessed = fun.scope.accesses.iter().any(|access| {
                if !matches!(&access.var_kind, VarKind::Global { .. }) {
                    return false;
                }
                let is_ref = arg.is_ref
                    && access
                        .info
                        .accessed_as
                        .to_lower()
                        .starts_with(arg.name.to_lower());
                is_ref || access.info.accessed_as.to_lower() == arg.name.to_lower()
            });
            if is_accessed {
                continue;
            }
            // Check if any function call in this function uses arg_name as a full-name
            let fun_match = section.fun_calls.iter().any(|fun_call| {
                let Depth::Nested { parent_fun } = fun_call.depth else {
                    // Call is not nested
                    return false;
                };
                fun_call.name_matches_global_arg
                    && parent_fun == fun.uuid
                    && fun_call.accessed_as.to_lower() == arg.name.to_lower()
            });

            if fun_match {
                continue;
            }

            let location = arg.location.as_ref().expect("Arg should have a Location");
            issues.add(
                IssueKind::UnusedModifierArg,
                location,
                format!(
                    "global variable `{}` is not used in {}()",
                    arg.name, fun.name
                ),
            );
        }
    }
}

fn unused_in_instance_mods(
    modifiers: &Vec<Modifier>,
    fun: &Fun,
    section: &Section,
    issues: &mut IssueTracker,
) {
    // Instances are unused if they are not read or written to
    for modifier in modifiers {
        for arg in &modifier.args {
            let is_accessed = fun.scope.accesses.iter().any(|access| {
                let VarKind::Instance { .. } = &access.var_kind else {
                    return false;
                };
                let info = &access.info;
                (info.is_write() || info.is_read())
                    && info.accessed_as.to_lower().starts_with(arg.name.to_lower())
            });
            if is_accessed {
                continue;
            }
            // Check if any function call in this function uses arg_name as a full-name
            let fun_match = section.fun_calls.iter().any(|fun_call| {
                let Depth::Nested { parent_fun } = fun_call.depth else {
                    // Call is not nested
                    return false;
                };
                fun_call.name_matches_instance_arg
                    && parent_fun == fun.uuid
                    && fun_call.accessed_as.to_lower() == arg.name.to_lower()
            });

            if fun_match {
                continue;
            }

            let location = arg.location.as_ref().expect("Arg should have a Location");
            issues.add(
                IssueKind::UnusedModifierArg,
                location,
                format!(
                    "instance variable `{}` is not used in {}()",
                    arg.name, fun.name
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
    fn unused_local() {
        let source = indoc! {"
            @init
            function foo() local(bar) (
                0;
            );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedModifierArg));
    }

    #[test]
    fn unused_instance() {
        let source = indoc! {"
            @init
            function foo() instance(bar) (
                0;
            );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedModifierArg));
    }

    #[test]
    fn unused_global() {
        let source = indoc! {"
            @init
            function foo() global(bar) (
                0;
            );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedModifierArg));
    }

    #[test]
    fn used_local() {
        let source = indoc! {"
            @init
            function foo() local(bar) (
              bar = 1;
              _ = bar;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedModifierArg));
    }

    #[test]
    fn used_global() {
        let source = indoc! {"
            @init
            function foo() global(srate) (
              this = srate;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedModifierArg));
    }

    #[test]
    fn used_ref_global() {
        let source = indoc! {"
            @init
            function bar(a, b) (
                _ = a;
                _ = b;
            );
            function foo() global(ref*) (
                bar(ref.inner1, ref.inner2);
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedModifierArg));
    }

    #[test]
    fn has_global_match() {
        let source = indoc! {"
            @init
            function fun() (
              this.secret = 10;
            );
            function foo() global(bar.fun) (
              bar.fun();
            );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedModifierArg));
    }
    #[test]
    fn used_as_ref() {
        let source = indoc! {"
            @init
            function foo() local(bar)
            (
              gfx_measurestr(str, bar, _);
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedModifierArg));
    }
    #[test]
    fn used_as_instance_match() {
        let source = indoc! {"
            @init
            function nested() (
              this.bar = 10;
            );
            function on_instance() instance(nested) (
              nested();
            );
            function on_this() (
              this.nested();
            );

            z.on_instance(); // z.bar = 10
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedModifierArg));
    }
}
