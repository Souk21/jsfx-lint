use crate::{IssueKind, issue::IssueTracker, program::Program};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    let maximum = 40;
    for section in program.sections.values() {
        for fun_def in &section.fun_defs {
            if fun_def.args.len() > maximum {
                issues.add(
                    IssueKind::TooManyParams,
                    fun_def
                        .location
                        .as_ref()
                        .expect("All non-builtin function's arg should have a location."),
                    format!(
                        "Function '{}' has too many parameters ({}), the maximum is {maximum}.",
                        fun_def.name,
                        fun_def.args.len()
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // `indoc!` removes indentation from multiline string
    use crate::IssueKind;
    use crate::file::File;
    use indoc::indoc;

    #[test]
    pub fn almost_too_many() {
        let source = indoc! {"
            @init
            function b(a,a,a,a,a  a,a,a,a,a,  a,a,a,a,a  a,a,a,a,a,  a,a,a,a,a  a,a,a,a,a,  a,a,a,a,a,  a,a,a,a,a) (0);
                     b(1,1,1,1,1, 1,1,1,1,1,  1,1,1,1,1, 1,1,1,1,1,  1,1,1,1,1, 1,1,1,1,1,  1,1,1,1,1,  1,1,1,1,1);"
        };
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::TooManyParams));
    }

    #[test]
    pub fn too_many() {
        let source = indoc! {"
            @init
            function a(a,a,a,a,a  a,a,a,a,a,  a,a,a,a,a  a,a,a,a,a,  a,a,a,a,a  a,a,a,a,a,  a,a,a,a,a,  a,a,a,a,a, a) (0);
                     a(1,1,1,1,1, 1,1,1,1,1,  1,1,1,1,1, 1,1,1,1,1,  1,1,1,1,1, 1,1,1,1,1,  1,1,1,1,1,  1,1,1,1,1, 1);"
        };
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::TooManyParams));
    }
}
