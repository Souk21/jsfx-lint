use crate::functions::ModifierKind;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (kind, modifiers) in &fun_def.modifiers {
                if !matches!(kind, ModifierKind::Local) {
                    continue;
                }
                for modifier in modifiers {
                    for arg in &modifier.args {
                        if arg.is_str {
                            issues.add(
                                IssueKind::StringArgInLocalMod,
                                arg.location.as_ref().expect(
                                    "All non-builtin function's arg should have a location.",
                                ),
                                String::from("Strings are not allowed in local()"),
                            );
                        }
                    }
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
    fn string_id_in_local() {
        let source = indoc! {"
            @init
            function foo() local(#lol) ( 0 );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::StringArgInLocalMod));
    }
}
