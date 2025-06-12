use crate::access::var_kind::VarKind;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (i, arg) in fun_def.args.iter().enumerate() {
                if !arg.is_ref {
                    continue;
                }
                let used_as_a_ref = fun_def.scope.accesses.iter().any(|access| {
                    matches!(&access.var_kind, VarKind::RefArg {arg_index, suffix} if *arg_index == i && !suffix.is_empty())
                });
                if used_as_a_ref {
                    continue;
                }
                let written = fun_def.scope.accesses.iter().any(|access| {
                    access.info.is_write()
                        && matches!(
                            &access.var_kind,
                            VarKind::RefArg { arg_index, suffix }
                                if *arg_index == i && suffix.is_empty()
                        )
                });
                if written {
                    // If it's written, it's not a useless ref
                    continue;
                }
                let read = fun_def.scope.accesses.iter().any(|access| {
                    access.info.is_read()
                        && matches!(
                            &access.var_kind,
                            VarKind::RefArg { arg_index, suffix }
                                if *arg_index == i && suffix.is_empty()
                        )
                });
                if !read {
                    // If it's not read (and not written), it's either an unused arg, which is reported elsewhere,
                    // Or it's only used as a namespace (`PassedByRef`), so it is not a useless ref
                    continue;
                }
                let location = arg.location.as_ref().expect("Arg should have a Location");
                issues.add(
                    IssueKind::UselessRefArg,
                    location,
                    format!("{} does not need to be ref", arg.name),
                );
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
    fn useless() {
        let source = indoc! {"
            @init
            function foo(a*) (
                _ = a;
            );
            foo(bar);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessRefArg));
    }

    #[test]
    fn not_useless_written() {
        let source = indoc! {"
            @init
            function foo(a*) (
                a = 0;
            );
            foo(bar);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessRefArg));
    }

    #[test]
    fn not_useless_obj_written() {
        let source = indoc! {"
            @init
            function foo(a*) (
                a.b = 0;
            );
            foo(bar);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessRefArg));
    }

    #[test]
    fn not_useless_obj_read() {
        let source = indoc! {"
            @init
            function foo(a*) (
                _ = a.b;
            );
            foo(bar);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessRefArg));
    }
}
