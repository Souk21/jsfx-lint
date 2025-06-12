use crate::functions::ModifierKind;
use crate::{IssueKind, Program, issue::IssueTracker};

/// Report empty modifier (except `global()`)
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            for (kind, modifiers) in &fun_def.modifiers {
                if matches!(kind, ModifierKind::Global) {
                    continue;
                }
                for modifier in modifiers {
                    if modifier.args.is_empty() {
                        issues.add(
                            IssueKind::EmptyModifier,
                            &modifier.location,
                            format!("Empty {}() for {}()", kind, fun_def.name),
                        );
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
    fn empty_modifier() {
        let source = indoc! {"
            @init
            function foo() local() ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::EmptyModifier));
    }
}
