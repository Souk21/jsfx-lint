use crate::access::var_kind::VarKind;
use crate::{IssueKind, Program, issue::IssueTracker};
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for access in &fun_def.scope.accesses {
                if matches!(access.var_kind, VarKind::Global{accessible} if !accessible) {
                    issues.add(
                        IssueKind::InaccessibleGlobal,
                        &access.info.location,
                        format!(
                            "Global variable {} is inaccessible",
                            access.info.accessed_as
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
    fn inaccessible_global() {
        let source = indoc! {"
            @init
            function foo() global(bar) ( baz = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InaccessibleGlobal));
    }
    #[test]
    fn inaccessible_global_with_empty_modifier() {
        let source = indoc! {"
            @init
            function foo() global() ( bar = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InaccessibleGlobal));
    }

    #[test]
    fn accessible_global() {
        let source = indoc! {"
            @init
            function foo() global(bar) ( bar = 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InaccessibleGlobal));
    }

    #[test]
    fn accessible_this_in_nested_fun() {
        let source = indoc! {"
            @init
            function foo() ( this.baz = 0; );
            function bar() global() ( foo() );
            bar();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InaccessibleGlobal));
    }

    #[test]
    fn accessible_arg_ref_in_nested_fun() {
        let source = indoc! {"
            @init
            function foo(baz*) ( baz = 0; );
            function bar() global(out) ( foo(out) );
            bar();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InaccessibleGlobal));
    }

    #[test]
    fn inaccessible_global_fun_call() {
        let source = indoc! {"
            @init
            function foo() (
                this.a = 1;
            );
            function bar() global(ref) (
                ref.foo();
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::InaccessibleGlobal));
    }

    #[test]
    fn accessible_global_fun_call() {
        let source = indoc! {"
            @init
            function foo() (
                this.a = 1;
            );
            function bar() global(ref*) (
                ref.foo();
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::InaccessibleGlobal));
    }

    #[test]
    fn accessible_global_match() {
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
        assert!(!issues.has(&IssueKind::InaccessibleGlobal));
    }
}
