use crate::functions::ModifierKind;
use crate::location::Location;
use crate::{
    IssueKind, Program,
    functions::{Arg, Modifier},
    issue::IssueTracker,
};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for arg in &fun_def.args {
                if arg.name.as_str().eq_ignore_ascii_case("this")
                    || arg.name.to_lower().starts_with("this.")
                {
                    issues.add(
                        IssueKind::FullyShadowedArgument,
                        arg.location
                            .as_ref()
                            .expect("All non-builtin function's arg should have a location."),
                        format!(
                            "`{}()` arg `{}` is fully shadowed by the `this` keyword",
                            fun_def.name, arg.name
                        ),
                    );
                }
            }
            if let Some(local_mods) = &fun_def.modifiers.get(&ModifierKind::Local) {
                warn_shadowed_local_args(
                    local_mods.iter().flat_map(|modifier| &modifier.args),
                    &fun_def.args,
                    &fun_def.name,
                    issues,
                );
            }
            if let Some(instance_mods) = &fun_def.modifiers.get(&ModifierKind::Instance) {
                warn_shadowed_instance_args(
                    instance_mods.iter().flat_map(|modifier| &modifier.args),
                    &fun_def.args,
                    fun_def.modifiers.get(&ModifierKind::Local),
                    &fun_def.name,
                    issues,
                );
            }
            if let Some(global_mods) = &fun_def.modifiers.get(&ModifierKind::Global) {
                warn_shadowed_global_args(
                    global_mods.iter().flat_map(|modifier| &modifier.args),
                    &fun_def.args,
                    fun_def.modifiers.get(&ModifierKind::Local),
                    fun_def.modifiers.get(&ModifierKind::Instance),
                    &fun_def.name,
                    issues,
                );
            }
        }
    }
}

fn warn_shadowed_global_args<'a, I>(
    global_args: I,
    fun_args: &[Arg],
    local_mod: Option<&Vec<Modifier>>,
    instance_mod: Option<&Vec<Modifier>>,
    fn_name: &str,
    issues: &mut IssueTracker,
) where
    I: IntoIterator<Item = &'a Arg>,
{
    // Is Y shadowed by X ?
    //               |  fun(arg)  |    fun(arg*)   | local(arg) | instance(arg)
    // global(arg)   :   Fully    |      Fully     |    Fully   |    Fully
    // global(arg*)  :     No     |   Partially¹   |     No     |    Fully
    //
    //  ¹ In this case only `arg.` resolves to the global(arg*) because
    //    a fun argument can't end with a dot but a global ref argument can

    for global_arg in global_args {
        warn_mod_arg_shadowed_by_this_keyword(
            global_arg.name.as_str(),
            issues,
            global_arg
                .location
                .as_ref()
                .expect("All non-builtin function's arg should have a location."),
            &ModifierKind::Global,
        );
        let found_fully = fun_args.iter().any(|fun_arg| {
            !global_arg.is_ref && fun_arg.name.to_lower() == global_arg.name.to_lower()
        });

        if found_fully {
            let name = if global_arg.is_ref {
                format!("{}*", global_arg.name)
            } else {
                global_arg.name.to_string()
            };
            let location = global_arg
                .location
                .as_ref()
                .expect("All non-builtin function's arg should have a location.");
            issues.add(
                IssueKind::FullyShadowedArgument,
                location,
                format!("global arg `{name}` is fully shadowed by `{fn_name}()` arg `{name}`"),
            );
        } else {
            let found_partially = fun_args.iter().any(|fun_arg| {
                global_arg.is_ref
                    && fun_arg.is_ref
                    && fun_arg.name.to_lower() == global_arg.name.to_lower()
            });
            if found_partially {
                let location = global_arg
                    .location
                    .as_ref()
                    .expect("All non-builtin function's arg should have a location.");
                issues.add(
                    IssueKind::PartiallyShadowedArgument,
                    location,
                    format!(
                        "global arg `{}` is partially shadowed by `{}()` arg {}",
                        global_arg.name, fn_name, global_arg.name
                    ),
                );
            }
        }
        if let Some(instance_mods) = instance_mod {
            let found = instance_mods
                .iter()
                .flat_map(|m| &m.args)
                .any(|instance_arg| instance_arg.name.to_lower() == global_arg.name.to_lower());
            if found {
                let location = global_arg
                    .location
                    .as_ref()
                    .expect("All non-builtin function's arg should have a location.");
                issues.add(
                    IssueKind::FullyShadowedArgument,
                    location,
                    format!(
                        "global arg `{}` is fully shadowed by instance arg `{}`",
                        global_arg.name, global_arg.name
                    ),
                );
            }
        }
        if !global_arg.is_ref {
            if let Some(local_mods) = local_mod {
                let found = local_mods
                    .iter()
                    .flat_map(|m| &m.args)
                    .any(|a| a.name.to_lower() == global_arg.name.to_lower());
                if found {
                    let location = global_arg
                        .location
                        .as_ref()
                        .expect("All non-builtin function's arg should have a location.");
                    issues.add(
                        IssueKind::FullyShadowedArgument,
                        location,
                        format!(
                            "global arg `{}` is fully shadowed by local arg `{}`",
                            global_arg.name, global_arg.name
                        ),
                    );
                }
            }
        }
    }
}
fn warn_shadowed_local_args<'a, I>(
    local_args: I,
    fun_args: &[Arg],
    fn_name: &str,
    issues: &mut IssueTracker,
) where
    I: IntoIterator<Item = &'a Arg>,
{
    for arg in local_args {
        warn_mod_arg_shadowed_by_this_keyword(
            arg.name.as_str(),
            issues,
            arg.location
                .as_ref()
                .expect("All non-builtin function's arg should have a location."),
            &ModifierKind::Local,
        );
        let found = fun_args.iter().any(|a| a.name == arg.name);
        if !found {
            continue;
        }
        let location = arg
            .location
            .as_ref()
            .expect("All non-builtin function's arg should have a location.");
        issues.add(
            IssueKind::FullyShadowedArgument,
            location,
            format!(
                "local arg {} is fully shadowed by `{}()` arg `{}`",
                arg.name, fn_name, arg.name
            ),
        );
    }
}

fn warn_shadowed_instance_args<'a, I>(
    instance_args: I,
    fun_args: &[Arg],
    local_mod: Option<&Vec<Modifier>>,
    fn_name: &str,
    issues: &mut IssueTracker,
) where
    I: IntoIterator<Item = &'a Arg>,
{
    // Instance are only shadowed by a function arg if it's a ref arg
    // e.g. function foo(bar*) instance(bar) (...) // bar is shadowed
    for arg in instance_args {
        warn_mod_arg_shadowed_by_this_keyword(
            arg.name.as_str(),
            issues,
            arg.location
                .as_ref()
                .expect("All non-builtin function's arg should have a location."),
            &ModifierKind::Instance,
        );
        let found_fully = fun_args
            .iter()
            .any(|fun_arg| fun_arg.name == arg.name && fun_arg.is_ref);
        if found_fully {
            let location = arg
                .location
                .as_ref()
                .expect("All non-builtin function's arg should have a location.");
            issues.add(
                IssueKind::FullyShadowedArgument,
                location,
                format!(
                    "instance arg `{}` is fully shadowed by `{}()` arg `{}`",
                    arg.name, fn_name, arg.name
                ),
            );
        } else {
            // If fun arg is not ref, instance arg is only partially shadowed
            let found_partially = fun_args
                .iter()
                .any(|fun_arg| fun_arg.name == arg.name && !fun_arg.is_ref);
            if found_partially {
                let location = arg
                    .location
                    .as_ref()
                    .expect("All non-builtin function's arg should have a location.");
                issues.add(
                    IssueKind::PartiallyShadowedArgument,
                    location,
                    format!(
                        "instance arg `{}` is partially shadowed by {}() arg `{}`",
                        arg.name, fn_name, arg.name
                    ),
                );
            }
        }
        if let Some(local_mods) = local_mod {
            let found = local_mods
                .iter()
                .flat_map(|m| &m.args)
                .any(|a| a.name.to_lower() == arg.name.to_lower());
            if found {
                let location = arg
                    .location
                    .as_ref()
                    .expect("All non-builtin function's arg should have a location.");
                issues.add(
                    IssueKind::PartiallyShadowedArgument,
                    location,
                    format!(
                        "instance arg `{}` is partially shadowed by local arg `{}`",
                        arg.name, arg.name
                    ),
                );
            }
        }
    }
}

pub fn warn_mod_arg_shadowed_by_this_keyword(
    name: &str,
    issues: &mut IssueTracker,
    location: &Location,
    mod_kind: &ModifierKind,
) {
    if name.eq_ignore_ascii_case("this") || name.to_ascii_lowercase().starts_with("this.") {
        issues.add(
            IssueKind::FullyShadowedArgument,
            location,
            format!("{mod_kind} arg `{name}` is fully shadowed by the `this` keyword"),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::file::File;
    use indoc::indoc;

    #[test]
    fn this() {
        let source = indoc! {"
            @init
            function foo(this) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn this_dot() {
        let source = indoc! {"
            @init
            function foo(this.) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }

    #[test]
    fn local_this_dot() {
        let source = indoc! {"
            @init
            function foo() local(this.) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn local_this() {
        let source = indoc! {"
            @init
            function foo() local(this) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn local_by_arg() {
        let source = indoc! {"
            @init
            function foo(bar) local(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn instance_this_dot() {
        let source = indoc! {"
            @init
            function foo() instance(this.) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn instance_this() {
        let source = indoc! {"
            @init
            function foo() instance(this) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn instance_by_ref_arg() {
        let source = indoc! {"
            @init
            function foo(bar*) instance(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn instance_by_arg() {
        let source = indoc! {"
            @init
            function foo(bar) instance(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::PartiallyShadowedArgument));
    }
    #[test]
    fn instance_by_local() {
        let source = indoc! {"
            @init
            function foo() local(bar) instance(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::PartiallyShadowedArgument));
    }

    #[test]
    fn global_this_dot() {
        let source = indoc! {"
            @init
            function foo() global(this.) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_this() {
        let source = indoc! {"
            @init
            function foo() global(this) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_by_arg() {
        let source = indoc! {"
            @init
            function foo(bar) global(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_by_ref_arg() {
        let source = indoc! {"
            @init
            function foo(bar*) global(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_ref_by_ref_arg() {
        // Only partially shadowed because `bar.` is valid on global(bar*) but not foo(bar*)
        let source = indoc! {"
            @init
            function foo(bar*) global(bar*) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::PartiallyShadowedArgument));
    }
    #[test]
    fn global_ref_by_arg() {
        let source = indoc! {"
            @init
            function foo(bar) global(bar*) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&crate::IssueKind::FullyShadowedArgument));
        assert!(!issues.has(&crate::IssueKind::PartiallyShadowedArgument));
    }
    #[test]
    fn global_by_instance() {
        let source = indoc! {"
            @init
            function foo() instance(bar) global(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_ref_by_instance() {
        let source = indoc! {"
            @init
            function foo() instance(bar) global(bar*) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_by_local() {
        let source = indoc! {"
            @init
            function foo() local(bar) global(bar) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
    #[test]
    fn global_ref_by_local() {
        let source = indoc! {"
            @init
            function foo() local(bar) global(bar*) (0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&crate::IssueKind::PartiallyShadowedArgument));
        assert!(!issues.has(&crate::IssueKind::FullyShadowedArgument));
    }
}
