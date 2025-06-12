use std::{collections::HashMap, error::Error, rc::Rc};

use lrlex::{LRNonStreamingLexerDef, lrlex_mod};
use regex::Regex;

use crate::{
    File, IssueKind, RcSubString,
    issue::IssueTracker,
    location::{LineCol, Location},
    meta::parse_metas,
    section::Chunk,
};
use crate::{file::FindImportResult, iterators::lines::Lines};

use super::{Meta, Section};
// Bring the lexer for `eel2.l` into scope.
lrlex_mod!("../grammar/eel2.l");

/// Contains the parsed `Metas` and the `AST` for sections of a file and all its imports
#[derive(Debug)]
pub struct FirstPass {
    pub metas: Vec<Meta>,
    pub sections: HashMap<&'static str, Section>,
}

impl FirstPass {
    #[allow(dead_code)]
    pub fn print_ast(&self) {
        for section in self.sections.values() {
            section.print_ast(0);
        }
    }
    fn new(
        file: &Rc<File>,
        lexer_def: &LRNonStreamingLexerDef<lrlex::DefaultLexeme, u32>,
        issues: &mut IssueTracker,
    ) -> Self {
        let source = &file.processed;
        let (sections, first_section_pos) = parse_sections(source, file, issues, lexer_def);
        let metas = parse_metas(source, file, first_section_pos, issues);
        Self { metas, sections }
    }
    pub fn from_file_recursive(
        file: &Rc<File>,
        issues: &mut IssueTracker,
    ) -> Result<Self, Box<dyn Error>> {
        let lexer_def = eel2_l::lexerdef();
        let mut first_pass = Self::new(file, &lexer_def, issues);
        first_pass.follow_imports(file, issues)?;
        Ok(first_pass)
    }

    pub fn handle_imports(
        &mut self,
        file_full_path: &str,
        issues: &mut IssueTracker,
    ) -> Result<(), Box<dyn Error>> {
        for meta in &self.metas {
            let Meta::Import {
                path: import_path,
                location,
            } = meta
            else {
                continue;
            };
            let import_result = File::find_import(file_full_path, import_path)?;
            let imported_file = match import_result {
                FindImportResult::Found(path) => path,
                FindImportResult::NotFound => {
                    issues.add(
                        IssueKind::ImportNotFound,
                        location,
                        format!("Import not found: {import_path}"),
                    );
                    continue;
                }
            };
            let imported = Self::from_file_recursive(&imported_file, issues)?;
            for (kind, section) in imported.sections {
                match kind {
                    "init" => {
                        if let Some(base_init) = self.sections.get_mut("init") {
                            // Add each import chunks in the same order as they were imported (top to bottom)
                            base_init.chunks.extend(section.chunks);
                        } else {
                            // Entry file (or previously imported file) doesn't have an @init section
                            self.sections.insert("init", section);
                        }
                    }
                    other_kind => {
                        // Only import sections that is not yet present
                        if !self.sections.contains_key(other_kind) {
                            self.sections.insert(other_kind, section);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn follow_imports(
        &mut self,
        file: &Rc<File>,
        issues: &mut IssueTracker,
    ) -> Result<(), Box<dyn Error>> {
        let Some(file_full_path) = &file.full_path else {
            // Importing is only supported for "real" files (i.e. files that have a path)
            // That means no support for `import` with stdin/test strings
            return Ok(());
        };
        let mut entry_init_chunks = Vec::new();
        if let Some(init_section) = self.sections.get_mut("init") {
            // Swap `entry_init_chunks` and `init_section.chunks`
            entry_init_chunks = std::mem::replace(&mut init_section.chunks, entry_init_chunks);
        }
        self.handle_imports(file_full_path, issues)?;
        if let Some(init_section) = self.sections.get_mut("init") {
            init_section.chunks.extend(entry_init_chunks);
        }
        Ok(())
    }
}

fn try_get_chunk_of_kind(
    section_kind: &'static str,
    line: &RcSubString,
    section_source: &RcSubString,
    file: &Rc<File>,
    line_pos: usize,
    issues: &mut IssueTracker,
) -> Option<Chunk> {
    // Line must start with `@section_kind`
    if !line.contains_at_pos(section_kind, "@".len()) {
        return None;
    }
    // Immediately following should be either a space or the end of the line
    let next_char = line.char_at("@".len() + section_kind.len());
    if !matches!(next_char, Some(' ') | None) {
        return None;
    }
    let params = get_params(section_kind, line, file, line_pos, issues);
    Some(Chunk {
        source: section_source.clone(),
        params,
        ast: None,
        file: file.clone(),
        line_pos,
    })
}

fn get_params(
    section_kind: &'static str,
    line: &RcSubString,
    file: &Rc<File>,
    line_pos: usize,
    issues: &mut IssueTracker,
) -> Option<RcSubString> {
    let param_start = section_kind.len() + "@".len() + " ".len();
    let param_str = line.substr(param_start..);
    if section_kind == "gfx" {
        warn_gfx_params(&param_str, file, line_pos, param_start, line, issues);
        return Some(param_str);
    }
    if !param_str.trim().is_empty() && !param_str.trim().starts_with("//") {
        warn_extraneous_params(
            file,
            line_pos,
            param_start,
            line,
            &param_str,
            section_kind,
            issues,
        );
    }
    None
}

fn warn_extraneous_params(
    file: &Rc<File>,
    line_pos: usize,
    param_start: usize,
    line: &RcSubString,
    param_str: &RcSubString,
    section_kind: &'static str,
    issues: &mut IssueTracker,
) {
    // @section has parameters but it shouldn't
    let location = Location {
        section: None,
        file: file.clone(),
        line_col: LineCol {
            start_line: line_pos,
            end_line: line_pos,
            start_column: param_start + 1,
            end_column: line.len() + 1,
        },
    };
    issues.add(
        IssueKind::WrongSectionParam,
        &location,
        format!(
            "Section @{section_kind} requires 0 parameters but received '{}'",
            param_str.trim()
        ),
    );
}

fn warn_gfx_params(
    param_str: &RcSubString,
    file: &Rc<File>,
    line_pos: usize,
    param_start: usize,
    line: &RcSubString,
    issues: &mut IssueTracker,
) {
    let splits = param_str
        .split(' ')
        .filter(|s| !s.is_empty())
        .take_while(|split| !split.starts_with("//"));
    let splits_count = splits.count();
    if splits_count != 2 && splits_count != 0 {
        let location = Location {
            section: None,
            file: file.clone(),
            line_col: LineCol {
                start_line: line_pos,
                end_line: line_pos,
                start_column: param_start + 1,
                end_column: line.len() + 1,
            },
        };
        issues.add(IssueKind::WrongSectionParam, &location,
                   format!("@gfx requires 0 or 2 params but was called with {splits_count} params ('{param_str}')"));
    }
}

fn parse_sections(
    source: &RcSubString,
    file: &Rc<File>,
    issues: &mut IssueTracker,
    lexer_def: &LRNonStreamingLexerDef<lrlex::DefaultLexeme, u32>,
) -> (HashMap<&'static str, Section>, Option<usize>) {
    let mut sections = HashMap::new();

    // First section position is where Metas end
    let mut first_section_pos = None;
    let mut char_pos = 0;
    // (?m) is multiline flag, so that ^ matches start of each line
    let regex = Regex::new(r"(?m)^@").unwrap();
    'sections: for (line_idx, (line, line_len)) in Lines::new(source).enumerate() {
        if !line.starts_with('@') {
            char_pos += line_len;
            continue;
        }
        if first_section_pos.is_none() {
            first_section_pos = Some(char_pos);
        }
        let line_end = char_pos + line_len;
        let is_last_line = line_end >= source.len();
        // Find where this section ends
        let next_section_pos = if is_last_line {
            source.len()
        } else {
            // Skip the line were parsing now
            regex.find(&source[line_end..]).map_or_else(
                || source.len(),
                |m| {
                    // Don't forget to add the length that was skipped
                    m.start() + char_pos + line.len()
                },
            )
        };

        // Only the line with the @section declaration
        let section_header = source.substr(char_pos..char_pos + line.len());
        // All the EEL code
        let section_eel = source.substr(line_end..next_section_pos);

        for section_kind in crate::SECTION_KINDS {
            if let Some(mut chunk) = try_get_chunk_of_kind(
                section_kind,
                &section_header,
                &section_eel,
                file,
                line_idx,
                issues,
            ) {
                let section = sections
                    .entry(section_kind)
                    .or_insert_with(|| Section::new(section_kind));
                chunk.parse(lexer_def, section.kind, issues);
                section.chunks.push(chunk);
                char_pos += line_len;
                continue 'sections;
            }
        }
        // Didn't find any matching section type
        let section_name = line;
        let location = Location {
            section: None,
            file: file.clone(),
            line_col: LineCol {
                start_line: line_idx,
                end_line: line_idx,
                start_column: 1,
                end_column: section_name.len() + 1,
            },
        };
        issues.add(
            IssueKind::UnknownSection,
            &location,
            format!("Unknown section type '{section_name}'"),
        );
        char_pos += line_len;
    }
    (sections, first_section_pos)
}
