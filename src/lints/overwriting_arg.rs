use crate::access::var_kind::VarKind;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (arg_index, arg) in fun_def.args.iter().enumerate() {
                if arg.is_ref {
                    // Ref args are not considered as overwriting
                    continue;
                }
                for access in &fun_def.scope.accesses {
                    if !matches!(access.var_kind, VarKind::Arg { arg_index: idx } if idx == arg_index)
                    {
                        continue;
                    }
                    if access.info.is_write() {
                        issues.add(
                            IssueKind::OverwritingArg,
                            &access.info.location,
                            format!(
                                "Function {}() overwrites argument {} before reading it.",
                                fun_def.name, access.info.accessed_as
                            ),
                        );
                    }
                    break;
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
    fn overwriting_arg() {
        let source = indoc! {"
            @init
            function foo(bar) (
                bar = 1;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::OverwritingArg));
    }
    #[test]
    fn overwriting_ref_arg_no_warning() {
        let source = indoc! {"
            @init
            function foo(bar*) (
                bar = 1;
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::OverwritingArg));
    }
}
