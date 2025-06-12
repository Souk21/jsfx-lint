#![cfg(test)]

use crate::{access, file::File};
use indoc::indoc;

#[test]
fn fun_def_prefix() {
    let source = indoc! {"
            @init
            function foo.baz() (
                this.bar = 10;
                _;
            );
            foo.baz();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("foo.bar"));
    let var = program
        .scope
        .variables
        .get("foo.bar")
        .expect("Variable 'foo.bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn fun_def_prefix_with_fun_call_prefix() {
    let source = indoc! {"
            @init
            function foo.baz() (
                this.bar = 10;
                _;
            );
            bin.foo.baz();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("bin.foo.bar"));
    let var = program
        .scope
        .variables
        .get("bin.foo.bar")
        .expect("Variable 'bin.foo.bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn matches_instance_arg() {
    let source = indoc! {"
            @init
            function nested() (
                this.bar = 10;
            );
            function foo() instance(nested) (
                nested()
            );
        "};
    let (program, _) = File::lint_with_default_config(source);
    let fun_call = program
        .sections
        .get("init")
        .unwrap()
        .fun_calls
        .first()
        .unwrap();
    assert!(fun_call.name_matches_instance_arg);
}

#[test]
fn non_match_instance_arg() {
    let source = indoc! {"
            @init
            function nested() (
                this.bar = 10;
            );
            function foo() instance(nest) (
                nested()
            );
        "};
    let (program, _) = File::lint_with_default_config(source);
    let fun_call = program
        .sections
        .get("init")
        .unwrap()
        .fun_calls
        .first()
        .unwrap();
    assert!(!fun_call.name_matches_instance_arg);
}

#[test]
fn with_prefix() {
    let source = indoc! {"
            @init
            function nested() (
                this.bar = 10;
            );
            function foo() instance(inner.nested) (
                inner.nested()
            );
        "};
    let (program, _) = File::lint_with_default_config(source);
    let fun_call = program
        .sections
        .get("init")
        .unwrap()
        .fun_calls
        .first()
        .unwrap();
    assert!(fun_call.name_matches_instance_arg);
}

#[test]
fn partial() {
    let source = indoc! {"
            @init
            function nested() (
                this.bar = 10;
            );
            function foo() instance(inner) (
                inner.nested()
            );
        "};
    let (program, _) = File::lint_with_default_config(source);
    let fun_call = program
        .sections
        .get("init")
        .unwrap()
        .fun_calls
        .first()
        .unwrap();
    assert!(fun_call.name_matches_instance_arg);
}
#[test]
fn almost_partial() {
    let source = indoc! {"
            @init
            function nested() (
                this.bar = 10;
            );
            function foo() instance(inner.) (
                inner.nested()
            );
        "};
    let (program, _) = File::lint_with_default_config(source);
    let fun_call = program
        .sections
        .get("init")
        .unwrap()
        .fun_calls
        .first()
        .unwrap();
    assert!(!fun_call.name_matches_instance_arg);
}
