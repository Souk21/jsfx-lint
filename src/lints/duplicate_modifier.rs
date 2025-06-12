use crate::functions::ModifierKind;
use crate::{IssueKind, Program, functions::Fun, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            warn_duplicate_mod(fun_def, &ModifierKind::Local, issues);
            warn_duplicate_mod(fun_def, &ModifierKind::Global, issues);
            warn_duplicate_mod(fun_def, &ModifierKind::Instance, issues);
        }
    }
}

fn warn_duplicate_mod(fun_def: &Fun, kind: &ModifierKind, issues: &mut IssueTracker) {
    if let Some(modifiers) = fun_def.modifiers.get(kind) {
        if modifiers.len() > 1 {
            issues.add(
                IssueKind::DuplicateModifier,
                &modifiers[1].location,
                format!("Duplicate {}() modifier for {}()", kind, fun_def.name),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn duplicate_modifier() {
        let source = indoc! {"
            @init
            function foo() local() local() ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateModifier));
    }

    #[test]
    fn duplicate_modifier_with_equivalent_names() {
        let source = indoc! {"
            @init
            function foo() static() local() ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::DuplicateModifier));
    }
}
