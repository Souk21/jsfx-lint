use std::collections::HashMap;

use crate::location::Location;
use crate::{IssueKind, Program, issue::IssueTracker};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    let mut function_usages = HashMap::new();
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            let Some(called_fun) = &fun_call.fun else {
                // Unknown function
                continue;
            };
            let entry = function_usages.entry(called_fun.uuid).or_insert(0);
            *entry += 1;
        }
    }
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            if function_usages.contains_key(&fun_def.uuid) {
                // Function is called at least once
                continue;
            }
            let Some(location @ Location { file, .. }) = &fun_def.location else {
                continue;
            };
            // Ignore unused functions from imports
            if !file.is_entry {
                continue;
            }
            issues.add(
                IssueKind::UnusedFunction,
                location,
                format!("function {}() is never used", fun_def.name),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn unused_function() {
        let source = indoc! {"
            @init
            function foo() ( 0; );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedFunction));
    }
}
