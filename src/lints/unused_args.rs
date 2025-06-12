use crate::access::var_kind::VarKind;
use crate::{IssueKind, Program, access, issue::IssueTracker};

/// Report unused args
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            if fun_def.is_builtin {
                continue;
            }
            let arg_accesses: Vec<_> = fun_def
                .scope
                .accesses
                .iter()
                .filter(|access::WithinFunction { var_kind, .. }| {
                    matches!(var_kind, VarKind::Arg { .. } | VarKind::RefArg { .. })
                })
                .collect();
            for (i, arg) in fun_def.args.iter().enumerate() {
                let mut read = false;
                let mut written = false;
                for access in &arg_accesses {
                    let access::WithinFunction {
                        var_kind: VarKind::RefArg { arg_index, .. } | VarKind::Arg { arg_index, .. },
                        ..
                    } = access
                    else {
                        continue;
                    };
                    if *arg_index != i {
                        continue;
                    }
                    match access.info.kind {
                        access::Kind::Read => read = true,
                        access::Kind::Write { .. } => written = true,
                        access::Kind::PassedByRef => (),
                    }
                }
                let unused_ref = arg.is_ref && !written && !read;
                let unread_arg = !arg.is_ref && !read;
                let unused = unused_ref || unread_arg;
                if unused {
                    let location = arg.location.as_ref().expect("Arg should have a Location");
                    issues.add(
                        IssueKind::ArgNeverRead,
                        location,
                        format!(
                            "argument `{}` is not used in `{}()`",
                            arg.name, fun_def.name
                        ),
                    );
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
    fn unused_arg() {
        let source = indoc! {"
            @init
            function foo(bar) ( 0; );
            foo(0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ArgNeverRead));
    }

    #[test]
    fn used_ref_arg() {
        let source = indoc! {"
            @init
            function foo(bar*) ( bar = 0; );
            foo(baz);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::ArgNeverRead));
    }
}
