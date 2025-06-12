use std::rc::Rc;

use cfgrammar::TIdx;
use lrlex::LRNonStreamingLexerDef;
use lrpar::{LexParseError, Lexeme, lrpar_mod};

use crate::IssueKind;
use crate::issue::IssueTracker;
use crate::iterators::ast_walk_signal::{AstWalkSignal, WalkSignal};
use crate::location::{LineCol, Location};
use crate::parser::{ParseError, ParserParam};
use crate::section::{Chunk, get_repair_str};
// Brings the parser for `eel2.y` into scope.
lrpar_mod!("../grammar/eel2.y");
impl Chunk {
    pub fn parse(
        &mut self,
        lexer_def: &LRNonStreamingLexerDef<lrlex::DefaultLexeme, u32>,
        section_kind: &'static str,
        issues: &mut IssueTracker,
    ) {
        let lexer = lexer_def.lexer(&self.source);
        let params = ParserParam {
            file: self.file.clone(),
            section_line_pos: self.line_pos,
            section_kind,
            source: self.source.clone(),
        };
        // Lex and parse the input
        let (res, errs) = eel2_y::parse(&lexer, &params);
        for e in errs {
            if let LexParseError::ParseError(er) = &e {
                self.handle_lex_parse_error(er, section_kind, issues);
            }
        }
        match res {
            Some(Ok(r)) => {
                self.ast = Some(r);
            }
            // Errors from eel2.y
            Some(Err(e)) => match &e {
                ParseError::ExpectedWhile(location)
                | ParseError::ExpectedFunModifier(location)
                | ParseError::ExpectedFunction(location) => {
                    issues.add(IssueKind::ParseError, location, e.to_string());
                }
                ParseError::DefaultLexeme(_) => {
                    // Parser sometimes throws this error when it can't recover from a parsing error
                    // (e.g. "{" and "}")
                    // The parser should return errors in `errs`, which will be dealt with in `handle_lex_parse_error`
                    // However this is not always the case, hence the reporting here too.
                    let location = Location {
                        file: self.file.clone(),
                        section: Some(section_kind),
                        line_col: LineCol {
                            start_line: self.line_pos,
                            end_line: self.line_pos,
                            start_column: 1,
                            end_column: 1,
                        },
                    };
                    issues.add(IssueKind::ParseError, &location, e.to_string());
                }
            },
            _ => {
                let location = Location {
                    file: self.file.clone(),
                    section: Some(section_kind),
                    line_col: LineCol {
                        start_line: self.line_pos,
                        end_line: self.line_pos,
                        start_column: 1,
                        end_column: 1,
                    },
                };
                issues.add(
                    IssueKind::ParseError,
                    &location,
                    String::from("Unrecoverable parser error"),
                );
            }
        }
    }

    fn handle_lex_parse_error(
        &self,
        er: &lrpar::ParseError<lrlex::DefaultLexeme, u32>,
        section_kind: &'static str,
        issues: &mut IssueTracker,
    ) {
        let repair_str = get_repair_str(er);
        let lexeme_start = er.lexeme().span().start();
        let (line, offset) = if lexeme_start == self.source.len() {
            let (last_line_idx, last_line) = self
                .source
                .lines()
                .enumerate()
                .last()
                .expect("source should have at least 1 line.");
            (last_line_idx + 1, last_line.len())
        } else {
            self.source
                .line_pos_at_pos(lexeme_start)
                .expect("lexeme start should be in source.")
        };
        let location = Location {
            file: Rc::clone(&self.file),
            section: Some(section_kind),
            line_col: LineCol {
                start_line: self.line_pos + line - 1,
                end_line: self.line_pos + line - 1,
                start_column: offset + 1,
                end_column: offset + 2,
            },
        };
        issues.add(
            IssueKind::ParseError,
            &location,
            format!(
                "Unexpected token {:?}. {}",
                eel2_y::token_epp(TIdx(er.lexeme().tok_id())).unwrap_or(""),
                repair_str
            ),
        );
    }

    pub fn print_ast(&self, indent: usize) {
        let Some(root) = &self.ast else {
            return;
        };
        let mut variable_indent = indent + 3;
        let indent = " ".repeat(indent);
        println!("{indent}Chunk:");
        for ref sig @ WalkSignal::Enter(ast_loc) | ref sig @ WalkSignal::Exit(ast_loc) in
            AstWalkSignal::new(root)
        {
            let indent_plus = " ".repeat(variable_indent);
            if matches!(sig, WalkSignal::Enter(_)) {
                println!("{indent_plus}{ast_loc}");
                variable_indent += 3;
            } else {
                variable_indent -= 3;
            }
        }
    }
}
