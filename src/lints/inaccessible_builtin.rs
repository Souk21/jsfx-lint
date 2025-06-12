use crate::{IssueKind, Program, issue::IssueTracker, variables::IsBuiltin};

/// Report writing to non-writable, reading non-readable
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for (key, variable) in &program.scope.variables {
        let IsBuiltin::BuiltIn(builtin) = program.scope.is_builtin(key, &program.metas) else {
            continue;
        };
        // There are no non-readable variable currently
        if !builtin.readable && variable.is_read() {
            // Can unwrap() because variable is read
            let location = &variable.first_read().unwrap().info.location;
            issues.add(
                IssueKind::ReadUnreadable,
                location,
                format!("Reading from non-readable variable {key}"),
            );
        }
        if !builtin.writable && variable.is_written() {
            // Can unwrap() because variable is written
            let location = &variable.first_write().unwrap().info.location;
            issues.add(
                IssueKind::WriteToNonWritable,
                location,
                format!("Writing to non-writable variable {key}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn write_to_non_writable() {
        let source = indoc! {"
            @init
            beat_position = 0;
            "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WriteToNonWritable));
    }

    #[test]
    fn write_to_non_writable_in_fn() {
        let source = indoc! {"
            @init
            function foo() ( beat_position = 0; );
            foo();
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WriteToNonWritable));
    }
}
