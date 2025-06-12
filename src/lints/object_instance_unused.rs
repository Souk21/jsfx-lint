use cmp::min;
use std::cmp;
use std::collections::HashSet;
use std::rc::Rc;
use uuid::Uuid;

use crate::access::var_kind::VarKind;
use crate::functions::Fun;
use crate::variables::{MaybeBoundToSlider, is_bound_to_slider};
use crate::{IssueKind, Program, access, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    let mut already_checked = HashSet::new();
    let mut already_warned_variable = HashSet::new();
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            check_fun_def(
                program,
                fun_def,
                &mut already_checked,
                &mut already_warned_variable,
                issues,
            );
        }
    }
}

fn check_fun_def(
    program: &Program,
    fun_def: &Rc<Fun>,
    already_checked: &mut HashSet<Uuid>,
    already_warned_variable: &mut HashSet<Uuid>,
    issues: &mut IssueTracker,
) {
    if !program.has_top_level_calls(fun_def) {
        // Only function that are called at the top-level are considered because their scope includes
        // the accesses of their nested functions.
        // In other words, only top-level function calls can mutate the global scope.
        return;
    }
    'outer: for fun_access in &fun_def.scope.accesses {
        if !matches!(
            &fun_access.var_kind,
            VarKind::Instance { .. } | VarKind::This { .. } | VarKind::RefArg { .. }
        ) {
            // Not a ref arg, instance or this access
            continue;
        }
        already_checked.insert(fun_access.uuid);
        let fun_access_origin = fun_access
            .origin
            .get_uuid()
            .expect("Variable is a ref/instance/this so it must have an origin");
        if *fun_access_origin != fun_access.uuid {
            // That means that this fun access is from a nested function
            continue;
        }
        let is_read_access = match fun_access.info.kind {
            access::Kind::Read => true,
            access::Kind::Write { .. } => false,
            access::Kind::PassedByRef => continue,
        };

        let mut global_access_referring_to_origin = Vec::new();
        for global_var in program.scope.variables.values() {
            let mut is_accessed_by_origin = false;
            // Using a loop because profiling showed that this is faster than using `any()` here.
            for global_access in &global_var.accesses {
                if global_access.origin.get_uuid() == Some(fun_access_origin) {
                    is_accessed_by_origin = true;
                    break;
                }
            }
            if !is_accessed_by_origin {
                continue;
            }
            if !already_warned_variable.insert(global_var.uuid) {
                // Already warned about this variable
                continue 'outer;
            }

            for global_access in &global_var.accesses {
                if global_access.origin.get_uuid() != Some(fun_access_origin) {
                    // This access is not referring to the origin of the fun access
                    continue;
                }
                global_access_referring_to_origin.push(global_access);
                for variable in program.scope.variables.values() {
                    if matches!(
                        is_bound_to_slider(&program.metas, variable.name.as_str()),
                        MaybeBoundToSlider::Some(_)
                    ) {
                        continue 'outer;
                    }
                    if variable.name.to_lower() == global_access.info.accessed_as.to_lower()
                        && (!is_read_access && variable.is_read()
                            || is_read_access && variable.is_written())
                    {
                        continue 'outer;
                    }
                }
            }
        }
        report(
            fun_def,
            issues,
            fun_access,
            is_read_access,
            &global_access_referring_to_origin,
        );
    }
}

fn report(
    fun_def: &Rc<Fun>,
    issues: &mut IssueTracker,
    fun_access: &access::WithinFunction,
    is_read_access: bool,
    global_access_referring_to_origin: &[&access::TopLevel],
) {
    let var_kind_name = if let VarKind::Instance { .. } = fun_access.var_kind {
        format!("{}() instance variable", fun_def.name)
    } else if let VarKind::This { .. } = fun_access.var_kind {
        format!("{}()", fun_def.name)
    } else {
        format!("{}() ref argument", fun_def.name)
    };
    let mut names = String::new();
    // Report up to 5 names
    let end_idx = min(5, global_access_referring_to_origin.len());
    for (i, acc) in global_access_referring_to_origin[..end_idx]
        .iter()
        .enumerate()
    {
        names.push('`');
        names.push_str(acc.info.accessed_as.as_str());
        names.push('`');
        if i + 2 == global_access_referring_to_origin.len() && i + 1 < end_idx {
            names.push_str(" and ");
        } else if i + 1 < end_idx {
            names.push_str(", ");
        }
    }
    let ellipsis = if global_access_referring_to_origin.len() > 5 {
        let rest = global_access_referring_to_origin.len() - 5;
        let s = if rest > 1 { "s" } else { "" };
        format!(", and {rest} other{s}",)
    } else {
        String::new()
    };
    names.push_str(ellipsis.as_str());
    let verb = if global_access_referring_to_origin.len() > 1 {
        "are"
    } else {
        "is"
    };
    let message = if is_read_access {
        format!("{verb} never assigned to and {verb} always 0")
    } else {
        format!("{verb} never read")
    };
    let s = if global_access_referring_to_origin.len() > 1 {
        "s"
    } else {
        ""
    };
    issues.add(
        IssueKind::ObjectAccessUnused,
        &fun_access.info.location,
        format!(
            "variable{s} {names} {message} (accessed via {var_kind_name} `{}`)",
            fun_access.info.accessed_as
        ),
    );
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::IssueKind;
    use crate::file::File;

    #[test]
    fn not_read_this() {
        let source = indoc! {"
            @init
            function foo() (
                this.bar = 1;
                // This line prevents this.bar from being returned by the function (and thus read)
                0;
            );
            object1.foo();
            object2.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }

    #[test]
    fn not_written_this() {
        let source = indoc! {"
            @init
            function foo() (
                this.bar;
            );
            object1.foo();
            object2.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }

    #[test]
    fn this_no_prefix() {
        let source = indoc! {"
            @init
            function foo() (
                this.bar = 1;
                // This line prevents bar from being returned by the function (and thus read)
                0;
            );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }
    #[test]
    fn not_read_instance() {
        let source = indoc! {"
            @init
            function foo() instance(bar) (
                bar = 1;
                // This line prevents bar from being returned by the function (and thus read)
                0;
            );
            object1.foo();
            object2.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }
    #[test]
    fn not_written_instance() {
        let source = indoc! {"
            @init
            function foo() instance(bar) (
                bar;
            );
            object1.foo();
            object2.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }
    #[test]
    fn not_read_ref_arg() {
        let source = indoc! {"
            @init
            function foo(bar*) (
                bar.hello = 1;
                0;
            );
            foo(object1);
            foo(object2);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }

    #[test]
    fn not_written_ref_arg() {
        let source = indoc! {"
            @init
            function foo(bar*) (
                bar;
            );
            foo(object1);
            foo(object2);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }
    #[test]
    fn not_written_ref_obj() {
        let source = indoc! {"
            @init
            function foo(bar*) (
                bar.hello;
            );
            foo(object1);
            foo(object2);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }
    #[test]
    fn not_read_ref_obj() {
        let source = indoc! {"
            @init
            function foo(bar*) (
                bar.hello = 1;
                0;
            );
            foo(object1);
            foo(object2);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ObjectAccessUnused));
    }

    #[test]
    fn not_read_instance_no_prefix() {
        let source = indoc! {"
            @init
            function foo() instance(k)
            (
                k = _;
                _ = k;
            );
            function bar() instance(k)
            (
                k = _;
            );
            _ ? (
                object1.foo();
            ) : (
                object1.bar();
            );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::ObjectAccessUnused));
    }

    #[test]
    fn multiple() {
        let source = indoc! {"
            @init
            function foo() instance(bar) (
                _ = bar;
                _ = bar;
            );

            object1.foo();
            object2.foo();
            object3.foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert_eq!(issues.count(&IssueKind::ObjectAccessUnused), 1);
    }

    #[test]
    fn multiple2() {
        let source = indoc! {"
            @init
            function foo() instance(bar) (
                _ = bar;
            );
            function oof(object*) (
                _ = object.bar;
            );

            object1.foo();
            object2.foo();
            object3.foo();
            oof(object1);
            oof(object2);
            oof(object3);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert_eq!(issues.count(&IssueKind::ObjectAccessUnused), 1);
    }
    #[test]
    fn used_by_slider() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Foo
            @init
            function bar() (
                _ = this;
            );
            foo.bar();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::ObjectAccessUnused));
    }
}
