use crate::variables::{IsBuiltin, MaybeBoundToSlider, is_bound_to_slider};
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for variable in program.scope.variables.values() {
        if variable.name.as_str() == "_" {
            continue;
        }
        if variable.name.as_str() == "#" {
            // Ignore top-level temp strings
            continue;
        }
        // Slider are irrelevant here
        if let MaybeBoundToSlider::Some(..) =
            is_bound_to_slider(&program.metas, variable.name.as_str())
        {
            continue;
        }
        // Builtins are irrelevant here
        let is_builtin = program
            .scope
            .is_builtin(variable.name.as_str(), &program.metas);
        if matches!(
            is_builtin,
            IsBuiltin::BuiltIn(_) | IsBuiltin::Global | IsBuiltin::Slider(_)
        ) {
            continue;
        }
        if !variable.is_read() && !variable.is_written() {
            // Variable is only passed by ref (i.e. used as a ref arg, to be used as an object)
            continue;
        }
        if variable_has_instance_this_ref_origin(variable) {
            // Variable is accessed through this/ref/instance, so it's being taken care of somewhere else
            continue;
        }
        if !variable.is_read() {
            issues.add(
                IssueKind::GlobalNeverRead,
                variable.first_location(),
                format!("`{}` is never read", variable.name),
            );
        }
        if !variable.is_written() {
            issues.add(
                IssueKind::GlobalNeverWritten,
                variable.first_location(),
                format!("`{}` is never assigned to and is always 0", variable.name),
            );
        }
    }
}

fn variable_has_instance_this_ref_origin(variable: &crate::variables::Variable) -> bool {
    variable
        .accesses
        .iter()
        .any(|access| access.origin.get_uuid().is_some())
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn global_never_read() {
        let source = indoc! {"
            @init
            foo = 0;
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::GlobalNeverRead));
    }

    #[test]
    fn global_never_written() {
        let source = indoc! {"
            @init
            foo = bar;
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::GlobalNeverWritten));
    }

    #[test]
    fn builtin_no_warning() {
        let source = indoc! {"
            @init
            a = beat_position;
            a = a;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverWritten));
    }

    #[test]
    fn nested_this() {
        let source = indoc! {"
            @init
            function recv() ( this.inner = 1; );
            function get_internal() ( a = this.inner; );
            function get() ( this.get_internal() );
            a_prefix.recv();
            a_prefix.get();
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverWritten));
    }

    #[test]
    fn instance_never_accessed() {
        let source = indoc! {"
            @init
            function foo(ref*)  (
              ref = 0;
            );
            function bar() instance(instance) (
              foo(instance);
            );
            bar();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverWritten));
    }

    #[test]
    fn ignore_underscore() {
        let source = indoc! {"
            @init
            _ = 0;
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverRead));
    }

    #[test]
    fn object_is_accessed() {
        // Do not trigger `GlobalNeverWritten` for "hello" because it is passed by ref
        let source = indoc! {"
            @init
            function read_bar(a*) (
                _ = a.bar;
            );
            hello.bar = 10;
            read_bar(hello);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverWritten));
    }

    #[test]
    fn this() {
        let source = indoc! {"
            @init
            function foo() (
              this = 1;
            );
            bar.foo();
            _ = bar;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverWritten));
    }

    #[test]
    fn temp_string() {
        let source = indoc! {r#"
            @init
            sprintf(#, "Hello, World!")
        "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverRead));
    }

    #[test]
    fn sprintf_var() {
        let source = indoc! {r#"
            @init
            a = 10;
            sprintf(#, "Hello, %{a}!")
        "#};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::GlobalNeverRead));
    }
}
