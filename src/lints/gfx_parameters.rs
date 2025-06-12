use crate::location::{LineCol, Location};
use crate::{IssueKind, Program, issue::IssueTracker};

/// Warn if @gfx parameters are not number (warning about param count is done somewhere else)
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    let Some(section) = program.sections.get("gfx") else {
        // No @gfx section
        return;
    };
    for chunk in &section.chunks {
        let Some(params) = &chunk.params else {
            continue;
        };
        let splits = params
            .as_str()
            .trim()
            .split(' ')
            .filter(|s| !s.is_empty())
            .take_while(|split| !split.starts_with("//"));
        for split in splits {
            let parsed = split.parse::<u64>();
            if parsed.is_err() {
                let location = Location {
                    file: chunk.file.clone(),
                    section: Some(section.kind),
                    line_col: LineCol {
                        start_line: chunk.line_pos,
                        start_column: 1,
                        end_line: chunk.line_pos,
                        end_column: "@gfx".len() + 1,
                    },
                };
                issues.add(
                    IssueKind::WrongSectionParam,
                    &location,
                    format!("Expected a number for gfx parameter, found: {split}",),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn gfx_too_many_params() {
        let source = "@gfx 10 20 30";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongSectionParam));
    }

    #[test]
    fn gfx_too_few_params() {
        let source = "@gfx 10";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongSectionParam));
    }

    #[test]
    fn gfx_non_int_params() {
        let source = "@gfx 10.3 20";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongSectionParam));
    }

    #[test]
    fn gfx_without_param() {
        let source = "@gfx";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::WrongSectionParam));
    }

    #[test]
    fn gfx_correct() {
        let source = "@gfx 10 20";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::WrongSectionParam));
    }

    #[test]
    fn init_with_params() {
        let source = "@init 10";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::WrongSectionParam));
    }
    #[test]
    fn gfx_comment() {
        let source = "@gfx //----------------------------------------------------------------";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::WrongSectionParam));
    }
    #[test]
    fn gfx_comment_after_params() {
        let source = "@gfx 1053 142 // request horizontal/vertical heights (0 means dont care)";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::WrongSectionParam));
    }

    #[test]
    fn init_comment() {
        let source = "@init  //comment";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::WrongSectionParam));
    }
}
