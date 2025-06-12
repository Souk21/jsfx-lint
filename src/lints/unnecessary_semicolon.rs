use crate::{IssueKind, Program, issue::IssueTracker, located_ast::LocatedAst};
use crate::{ast::Ast, location::LineCol};
use crate::{iterators::ast_walk::AstWalk, location::Location};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for chunk in &section.chunks {
            let Some(root) = &chunk.ast else {
                // Chunk is not parsed
                continue;
            };
            for LocatedAst { ast, .. } in AstWalk::new(root) {
                let Ast::Compound {
                    extra_semicolon: Some((count, location)),
                    ..
                } = ast
                else {
                    continue;
                };
                if *count > 1 {
                    let new_location = Location {
                        line_col: LineCol {
                            start_line: location.line_col.start_line,
                            start_column: location.line_col.start_column + 1,
                            end_line: location.line_col.end_line,
                            end_column: location.line_col.end_column,
                        },
                        file: chunk.file.clone(),
                        section: location.section,
                    };
                    issues.add(
                        IssueKind::UnnecessarySemicolon,
                        &new_location,
                        "Unnecessary semicolon".into(),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::IssueKind;
    use crate::file::File;
    use indoc::indoc;

    #[test]
    fn ok() {
        let source = indoc! {"
            @init
            a;
            b;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnnecessaryComma));
    }

    #[test]
    fn unnecessary_semicolon() {
        let source = indoc! {"
            @init
            a;;
            b;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnnecessarySemicolon));
    }
}
