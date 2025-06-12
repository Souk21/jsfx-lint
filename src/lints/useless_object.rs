use crate::IssueKind;
use crate::access::var_kind::VarKind;
use crate::functions::Depth;
use crate::issue::IssueTracker;
use crate::program::Program;

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            let Some(_) = &fun_call.prefix else {
                // Function was not called on an object
                continue;
            };
            let Some(called_fun) = &fun_call.fun else {
                // Unknown function
                continue;
            };
            let any_instance = called_fun.scope.accesses.iter().any(|access| {
                matches!(
                    access.var_kind,
                    VarKind::Instance { .. } | VarKind::This { .. }
                )
            });
            if any_instance {
                // "this" or "instance()" are accessed
                continue;
            }
            let Some(called_fun_section) = called_fun
                .location
                .as_ref()
                .and_then(|location| location.section.as_ref())
            else {
                // Builtin function
                continue;
            };
            let called_fun_section = program
                .sections
                .get(called_fun_section)
                .expect("Called function section not found");
            let used_as_full_name_fun_call = called_fun_section.fun_calls.iter().any(|called_fun_section_fun_call| {
                let is_called_from_fun = matches!(called_fun_section_fun_call.depth, Depth::Nested { parent_fun } if parent_fun == called_fun.uuid);
                is_called_from_fun && called_fun_section_fun_call.name_matches_instance_arg
            });
            if used_as_full_name_fun_call {
                continue;
            }
            issues.add(IssueKind::UselessObject, &fun_call.location, format!("{}() does not need to have a prefix, as it does not access any instance/this variables.", called_fun.name));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::IssueKind;
    use crate::file::File;
    use indoc::indoc;

    #[test]
    fn useless_obj() {
        let source = indoc! {"
            @init
            function foo() ( 0; );
            bar.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UselessObject));
    }

    #[test]
    fn not_useless_this() {
        let source = indoc! {"
            @init
            function foo() ( this.baz = 0; );
            bar.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessObject));
    }

    #[test]
    fn not_useless_instance() {
        let source = indoc! {"
            @init
            function foo() instance(baz) ( baz = 0; );
            bar.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessObject));
    }

    #[test]
    fn not_useless_nested() {
        let source = indoc! {"
            @init
            function set_foo() ( this.foo = 0; );
            function bar() ( this.nested.set_foo() );
            object.bar();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessObject));
    }

    #[test]
    fn not_useless_instance_full_name() {
        let source = indoc! {"
            @init
            function nested() instance(bar) (
              bar = 10;
            );

            function foo() instance(nested) (
              nested();
            );

            plop.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UselessObject));
    }
}
