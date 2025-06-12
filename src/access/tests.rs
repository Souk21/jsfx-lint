#![cfg(test)]
use crate::access::{Origin, OriginDetails};
use crate::file::File;
use crate::{IssueKind, access};
use indoc::indoc;

#[test]
fn return_read() {
    let source = indoc! {"
            @init
            function unit(foo) (
                foo;
            );
            unit(a);
        "};
    let (_, issues) = File::lint_with_default_config(source);
    assert!(!issues.has(&IssueKind::ArgNeverRead));
}

#[test]
fn navigating_hierarchy_up() {
    let source = indoc! {"
            @init
            function foo() (
                this..bar = 10;
                _;
            );
            object.foo(); //bar = 10
            object.inner.foo(); //object.bar = 10
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("bar"));
    assert!(program.scope.variables.contains_key("object.bar"));
    let bar = program
        .scope
        .variables
        .get("bar")
        .expect("Variable 'bar' should exist");
    let object_bar = program
        .scope
        .variables
        .get("object.bar")
        .expect("Variable 'object.bar' should exist");
    assert_eq!(bar.accesses.len(), 1);
    assert_eq!(object_bar.accesses.len(), 1);
    assert!(matches!(
        bar.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
    assert!(matches!(
        object_bar.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}
#[test]
fn navigating_prefix_less_fun_call() {
    let source = indoc! {"
            @init
            function foo() (
              this..bar = 10;
              _;
            );
            foo();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("bar"));
    let bar = program
        .scope
        .variables
        .get("bar")
        .expect("Variable 'bar' should exist");
    assert_eq!(bar.accesses.len(), 1);
    assert!(matches!(
        bar.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn navigating_hierarchy_up_nested() {
    let source = indoc! {"
            @init
            function nested() (
              this..bar = 10;
              _;
            );
            function foo() (
              this...nested();
              _;
            );
            hello.world.how.are.you.foo(); // hello.world.bar = 10
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("hello.world.bar"));
    let var = program
        .scope
        .variables
        .get("hello.world.bar")
        .expect("Variable 'hello.world.bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn navigating_too_far() {
    let source = indoc! {"
            @init
            function nested() (
              this......bar = 10;
              _;
            );
            function foo() (
              this...nested();
              _;
            );
            hello.world.how.are.you.foo(); // bar = 10
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("bar"));
    let var = program
        .scope
        .variables
        .get("bar")
        .expect("Variable 'bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}
#[test]
fn navigating_nested_object() {
    let source = indoc! {"
            @init
            function nested() (
              this..bar = 10;
              _;
            );
            function foo() (
              object.inner.nested();
              _;
            );
            foo(); //object.bar = 10;
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("object.bar"));
    let var = program
        .scope
        .variables
        .get("object.bar")
        .expect("Variable 'object.bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn various_this_navigation() {
    let source = indoc! {"
            @init
            function nested() (
              this.X = 1234;
              this..Y = 1234;
            );
            function inter() (
              this.foo.nested(); // obj1.foo.X obj1.Y
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("obj1.foo.x"));
    assert!(program.scope.variables.contains_key("obj1.y"));
}
#[test]
fn various_this_navigation2() {
    let source = indoc! {"
            @init
            function nested() (
              this.X = 1234;
              this..Y = 1234;
            );
            function inter() (
              this..nested(); // nested.X Y
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("nested.x"));
    assert!(program.scope.variables.contains_key("y"));
}
#[test]
fn various_this_navigation3() {
    let source = indoc! {"
            @init
            function nested() (
              this.X = 1234;
              this..Y = 1234;
            );
            function inter() (
              this...nested(); // nested.X Y
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("nested.x"));
    assert!(program.scope.variables.contains_key("y"));
}
#[test]
fn various_this_navigation4() {
    let source = indoc! {"
            @init
            function nested() (
              this.X = 1234;
              this..Y = 1234;
            );
            function inter() (
              this..foo.nested(); // foo.X Y
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("foo.x"));
    assert!(program.scope.variables.contains_key("y"));
}
#[test]
fn various_this_navigation5() {
    let source = indoc! {"
            @init
            function nested() (
              this.X = 1234;
              this..Y = 1234;
            );
            function inter() (
              this...foo.nested(); // foo.X Y
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("foo.x"));
    assert!(program.scope.variables.contains_key("y"));
}
#[test]
fn various_this_navigation6() {
    let source = indoc! {"
            @init
            function nested() instance(X) (
              X = 1234;
            );
            function inter() (
              this.foo.nested(); // obj1.foo.X
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("obj1.foo.x"));
}
#[test]
fn various_this_navigation7() {
    let source = indoc! {"
            @init
            function nested() instance(X) (
              X = 1234;
            );
            function inter() (
              this..nested(); // nested.X
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("nested.x"));
}
#[test]
fn various_this_navigation8() {
    let source = indoc! {"
            @init
            function nested() instance(X) (
              X = 1234;
            );
            function inter() (
              this...nested(); // nested.X
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("nested.x"));
}
#[test]
fn various_this_navigation9() {
    let source = indoc! {"
            @init
            function nested() instance(X) (
              X = 1234;
            );
            function inter() (
              this..foo.nested(); // foo.X
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("foo.x"));
}
#[test]
fn various_this_navigation10() {
    let source = indoc! {"
            @init
            function nested() instance(X) (
              X = 1234;
            );
            function inter() (
              this...foo.nested(); // foo.X
            );
            obj1.inter();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("foo.x"));
}
#[test]
fn navigating_inner_this() {
    let source = indoc! {"
            @init
            function nested() (
              this..bar = 10;
              _;
            );
            function foo() (
              this.inner.nested();
              _;
            );
            object.foo(); //object.bar = 10;
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("object.bar"));
    let var = program
        .scope
        .variables
        .get("object.bar")
        .expect("Variable 'object.bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn instance_full_name() {
    let source = indoc! {"
            @init
            function nested() (
              this.bar = 10;
              _;
            );
            function on_instance() instance(nested) (
              nested();
              _;
            );
            function on_this() (
              this.nested();
              _;
            );
            z.on_instance(); // z.bar = 10
            y.on_this(); // y.bar = 10
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("z.bar"));
    assert!(program.scope.variables.contains_key("y.bar"));
    let var_z = program
        .scope
        .variables
        .get("z.bar")
        .expect("Variable 'z.bar' should exist");
    let var_y = program
        .scope
        .variables
        .get("y.bar")
        .expect("Variable 'y.bar' should exist");
    assert_eq!(var_z.accesses.len(), 1);
    assert_eq!(var_y.accesses.len(), 1);
    assert!(matches!(
        var_z.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
    assert!(matches!(
        var_y.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn instance_full_name_prefix() {
    let source = indoc! {"
            @init
            function bar() (
                this.inner = 1;
                _;
            );
            function foo() instance(boo.bar) (
                boo.bar();
                _;
            );
            object.foo(); // object.boo.inner gets set
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(!program.scope.variables.contains_key("object.inner"));
    assert!(program.scope.variables.contains_key("object.boo.inner"));
    let var = program
        .scope
        .variables
        .get("object.boo.inner")
        .expect("Variable 'object.boo.inner' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn instance_partial_fun_call_match() {
    let source = indoc! {"
            @init
            function bar() (
                this.inner = 1;
                _;
            );
            function foo() instance(boo.a) (
                boo.a.e.bar(); // this.boo.a.bar()
                _;
            );
            object.foo(); // object.boo.a.e.inner gets set
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("object.boo.a.e.inner"));
    let var = program
        .scope
        .variables
        .get("object.boo.a.e.inner")
        .expect("Variable 'object.boo.a.e.inner' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn instance_matches_with_instance_access() {
    let source = indoc! {"
            @init
            function nested() instance(bar) (
              bar = 10;
              _;
            );
            function foo() instance(nested) (
              nested();
              _;
            );
            object.foo();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("object.bar"));
    let var = program
        .scope
        .variables
        .get("object.bar")
        .expect("Variable 'object.bar' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn instance_matches_with_instance_access_inner() {
    let source = indoc! {"
            @init
            function nested() instance(bar) (
              bar.inner = 10;
              _;
            );
            function foo() instance(nested) (
              nested();
              _;
            );
            object.foo();
        "};
    let (program, _) = File::lint_with_default_config(source);
    assert!(program.scope.variables.contains_key("object.bar.inner"));
    let var = program
        .scope
        .variables
        .get("object.bar.inner")
        .expect("Variable 'object.bar.inner' should exist");
    assert_eq!(var.accesses.len(), 1);
    assert!(matches!(
        var.accesses[0].info.kind,
        access::Kind::Write { .. }
    ));
}

#[test]
fn origin() {
    let source = indoc! {"
            @init
            function foo() (
                this.bar = 10;
                0;
            );
            function bob() (
                this.foo();
                0;
            );
            object.bob();
        "};
    let (program, _) = File::lint_with_default_config(source);
    let init = program
        .sections
        .get("init")
        .expect("@init section should exist");
    let bob = init
        .fun_defs
        .iter()
        .find(|fun| fun.name.as_str() == "bob")
        .unwrap();
    let foo = init
        .fun_defs
        .iter()
        .find(|fun| fun.name.as_str() == "foo")
        .unwrap();
    let bob_access = &bob.scope.accesses[0];
    let foo_access = &foo.scope.accesses[0];
    let top_access = &program
        .scope
        .variables
        .get("object.bar")
        .expect("Variable 'object.bar' should exist")
        .accesses[0];
    assert!(
        matches!(foo_access.origin, Origin::This(OriginDetails {uuid,..}) if uuid == foo_access.uuid)
    );
    assert!(
        matches!(bob_access.origin, Origin::This(OriginDetails {uuid,..}) if uuid == foo_access.uuid)
    );
    assert!(
        matches!(top_access.origin, Origin::This(OriginDetails {uuid,..}) if uuid == foo_access.uuid)
    );
}
