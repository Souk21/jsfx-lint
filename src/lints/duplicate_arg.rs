use crate::functions::Modifier;
use crate::functions::ModifierKind;
use crate::{IssueKind, Program, issue::IssueTracker};
use std::collections::HashMap;

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            // In argument list
            for (i, arg) in fun_def.args.iter().enumerate() {
                for other_arg in fun_def.args.iter().skip(i + 1) {
                    if arg.name.to_lower() == other_arg.name.to_lower()
                        && arg.is_ref == other_arg.is_ref
                    {
                        issues.add(
                            IssueKind::DuplicateArgument,
                            other_arg
                                .location
                                .as_ref()
                                .expect("All non-builtin function's arg should have a location."),
                            format!("duplicate argument `{}`", arg.name),
                        );
                    }
                }
            }
            // In modifiers
            warn_duplicate_arg_in_modifier(&ModifierKind::Local, &fun_def.modifiers, issues);
            warn_duplicate_arg_in_modifier(&ModifierKind::Global, &fun_def.modifiers, issues);
            warn_duplicate_arg_in_modifier(&ModifierKind::Instance, &fun_def.modifiers, issues);
        }
    }
}

pub fn warn_duplicate_arg_in_modifier(
    kind: &ModifierKind,
    modifiers: &HashMap<ModifierKind, Vec<Modifier>>,
    issues: &mut IssueTracker,
) {
    let Some(modifier) = modifiers.get(kind) else {
        return;
    };
    let args: Vec<_> = modifier
        .iter()
        .flat_map(|modifier| &modifier.args)
        .collect();

    for (i, arg) in args.iter().enumerate() {
        for other_arg in args.iter().skip(i + 1) {
            if arg.name.to_lower() == other_arg.name.to_lower() && arg.is_ref == other_arg.is_ref {
                issues.add(
                    IssueKind::DuplicateArgument,
                    other_arg
                        .location
                        .as_ref()
                        .expect("All non-builtin function's arg should have a location."),
                    format!("duplicate {kind} argument: `{}`", arg.name),
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
    fn duplicate_arg_in_args() {
        let source = indoc! {"
            @init
            function foo(bar, bar) ( 0 );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateArgument));
    }

    #[test]
    fn duplicate_arg_in_modifier() {
        let source = indoc! {"
            @init
            function foo() local(bar, bar) ( 0 );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateArgument));
    }

    #[test]
    fn duplicate_arg_in_multiple_modifiers() {
        let source = indoc! {"
        @init
        function foo() local(bar) local(bar) ( 0 );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateArgument));
    }

    #[test]
    fn duplicate_arg_in_modifier_with_varying_case() {
        let source = indoc! {"
            @init
            function foo() local(bar, Bar) ( 0 );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateArgument));
    }

    #[test]
    fn duplicate_arg_in_args_with_varying_case() {
        let source = indoc! {"
            @init
            function foo(bar, Bar) ( 0 );"};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateArgument));
    }
}
