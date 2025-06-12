use crate::file::File;
use crate::iterators::lines::Lines;
use std::{ops::Range, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LineCol {
    /// 0-based, inclusive
    pub start_line: usize,
    /// 0-based, inclusive
    pub end_line: usize,
    /// 1-based, inclusive
    pub start_column: usize,
    /// 1-based, exclusive
    pub end_column: usize,
}

pub struct Printable<'a> {
    /// The line containing the start location:
    /// - from `processed` if the location is in a preprocessor block
    /// - from `raw` otherwise.
    pub line: &'a str,
    /// A string representing the location
    pub location_text: String,
    /// The line and column of the location, based on the same source (raw/processed) than `self.line`
    pub shown_line_col: LineCol,
    pub in_preprocessor: bool,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Position {
    InPreProcessorBlock { pp_block: Range<usize> },
    Normal(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    /// Position relative to the top of the processed source.
    /// Columns are 1-based, and lines are 0-based.
    /// `end_line` is inclusive and `end_column` is exclusive.
    pub line_col: LineCol,
    pub file: Rc<File>,
    /// Type of the containing `Section`.
    /// Will be `None` for `Meta`s, which are not in a section
    /// Possible values are in `SECTION_KINDS` in `globals.rs`.
    pub section: Option<&'static str>,
}

impl Location {
    /// Converts a character index in the processed source to a character index in the raw source.
    pub fn processed_char_idx_to_raw_char_idx(
        file: &File,
        position: usize,
    ) -> Result<Position, Box<dyn std::error::Error>> {
        // If there are no preprocessor blocks, the position is the same in both sources
        if file.pp_chunks.is_empty() {
            return Ok(Position::Normal(position));
        }
        // If the position is in a preprocessor block output, return the block range
        for pp_chunk in &file.pp_chunks {
            if pp_chunk.processed.contains(&position) {
                return Ok(Position::InPreProcessorBlock {
                    pp_block: pp_chunk.raw.clone(),
                });
            }
        }
        // Otherwise, calculate the position in the raw source
        let mut cur_position = isize::try_from(position)?;

        for pp_chunk in &file.pp_chunks {
            if position < pp_chunk.processed.start {
                break;
            }
            let delta =
                isize::try_from(pp_chunk.raw.len())? - isize::try_from(pp_chunk.processed.len())?;
            cur_position += delta;
        }
        Ok(Position::Normal(usize::try_from(cur_position)?))
    }

    /// Returns the line (0-based) and column (0-based) of a character index in a source string.
    pub fn char_idx_to_line_and_col(source: &str, position: usize) -> (usize, usize) {
        let mut line_start = 0;
        for (line_idx, (_, line_len)) in Lines::new(source).enumerate() {
            let line_end = line_start + line_len;
            if position >= line_start && position < line_end {
                return (line_idx, position - line_start);
            }
            line_start = line_end;
        }
        (0, 0)
    }

    pub fn char_range_to_line_col(source: &str, range: &std::ops::Range<usize>) -> LineCol {
        let (start_line, start_column) = Self::char_idx_to_line_and_col(source, range.start);
        let (end_line, end_column) = Self::char_idx_to_line_and_col(source, range.end);
        // `Line Col` columns are 1-based
        LineCol {
            start_line,
            start_column: start_column + 1,
            end_line,
            end_column: end_column + 1,
        }
    }

    pub fn to_processed_char_range(&self) -> std::ops::Range<usize> {
        let mut start_pos = 0;
        let mut end_pos = 0;
        for (line_idx, (_, line_len)) in Lines::new(&self.file.processed).enumerate() {
            let found_start = line_idx >= self.line_col.start_line;
            let found_end = line_idx >= self.line_col.end_line;
            if !found_start {
                start_pos += line_len;
            }
            if !found_end {
                end_pos += line_len;
            }
            if found_start && found_end {
                return start_pos + self.line_col.start_column - 1
                    ..end_pos + self.line_col.end_column - 1;
            }
        }
        panic!("Didn't find the line in the processed source");
    }

    pub fn to_printable(&self) -> Result<Printable, Box<dyn std::error::Error>> {
        let range = self.to_processed_char_range();
        let start = Self::processed_char_idx_to_raw_char_idx(&self.file, range.start)?;
        let end = Self::processed_char_idx_to_raw_char_idx(&self.file, range.end)?;
        match start {
            Position::InPreProcessorBlock { pp_block } => {
                // Location in preprocessor block returns the line from `processed` but the `line_col` from `raw`
                let processed_line_col = &self.line_col;
                // Use `processed` source for code in pp blocks, to return a line from the generated code
                let line = self
                    .file
                    .processed
                    .lines()
                    .nth(processed_line_col.start_line)
                    .expect("Line should be found in processed source");
                let raw_line_col = Self::char_range_to_line_col(&self.file.raw, &pp_block);
                // Use `raw` for the location text, so it points to the pp block
                let location_text = format!(
                    "{}:{}:{} (in preprocessed code)",
                    self.file.get_printable_path(),
                    raw_line_col.start_line + 1,
                    raw_line_col.start_column
                );
                Ok(Printable {
                    line,
                    location_text,
                    shown_line_col: processed_line_col.clone(),
                    in_preprocessor: true,
                })
            }
            Position::Normal(start_idx) => {
                if let Position::Normal(end_idx) = end {
                    // Use `raw` as a source for normal code
                    let source = &self.file.raw;
                    let range = start_idx..end_idx;
                    let raw_line_col = Self::char_range_to_line_col(source, &range);
                    let line = source.lines().nth(raw_line_col.start_line).unwrap();
                    // Use 'raw' for the location as well
                    let location_text = format!(
                        "{}:{}:{}",
                        self.file.get_printable_path(),
                        raw_line_col.start_line + 1,
                        raw_line_col.start_column
                    );
                    return Ok(Printable {
                        line,
                        location_text,
                        shown_line_col: raw_line_col,
                        in_preprocessor: false,
                    });
                }
                panic!("Start is normal code but end position is in preprocessor block");
            }
        }
    }

    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        // First compare file path, but keep the entry file last
        let file_cmp = if self.file.is_entry && !other.file.is_entry {
            std::cmp::Ordering::Greater
        } else if !self.file.is_entry && other.file.is_entry {
            std::cmp::Ordering::Less
        } else {
            self.file.full_path.cmp(&other.file.full_path)
        };

        if file_cmp != std::cmp::Ordering::Equal {
            return file_cmp;
        }

        // Then, if it's in the same file, compare by starting line
        let start_line_a = self.line_col.start_line;
        let start_line_b = other.line_col.start_line;
        let line_cmp = start_line_a.cmp(&start_line_b);
        if line_cmp != std::cmp::Ordering::Equal {
            return line_cmp;
        }
        // Then, if it's in the same line, compare by starting column
        self.line_col.start_column.cmp(&other.line_col.start_column)
    }
}

#[cfg(test)]
mod tests {
    use crate::location::{LineCol, Location};
    use crate::{file::File, location::Position};
    use std::rc::Rc;

    #[test]
    fn processed_to_raw_idx() -> Result<(), Box<dyn std::error::Error>> {
        let prelude = "a = 1;\n";
        let block = r#"<? printf("very_long_identifier = 2;"); ?>"#;
        let epilogue = "\nc = 3;";
        let expected_block = "very_long_identifier = 2;";
        let source = format!("{prelude}{block}{epilogue}");
        let file = File::from_str(&source);
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, 0)?,
            Position::Normal(0)
        );
        let prelude_last_char = prelude.len() - 1;
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, prelude_last_char)?,
            Position::Normal(prelude_last_char)
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, prelude_last_char + 1)?,
            Position::InPreProcessorBlock {
                pp_block: prelude.len()..prelude.len() + block.len()
            }
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, prelude_last_char + 2)?,
            Position::InPreProcessorBlock {
                pp_block: prelude.len()..prelude.len() + block.len()
            }
        );
        let block_end = prelude.len() + expected_block.len();
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, block_end)?,
            Position::Normal(prelude.len() + block.len())
        );
        Ok(())
    }

    #[test]
    fn lots_of_lines() -> Result<(), Box<dyn std::error::Error>> {
        let prelude = "a = 1;\n";
        let block = r#"<? loop(100, printf("b = 2;\n")); ?>"#;
        let epilogue = "\nc = 3;";
        let source = format!("{prelude}{block}{epilogue}");
        let expected_block = "b = 2;\n".repeat(100);
        let file = File::from_str(&source);
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, 0)?,
            Position::Normal(0)
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(
                &file,
                prelude.len() + expected_block.len()
            )?,
            Position::Normal(prelude.len() + block.len())
        );
        Ok(())
    }
    #[test]
    fn char_idx_to_line_and_col() {
        let line1 = "1234";
        let line2 = "56789";
        let line3 = "";
        let line4 = "abcd";
        let source = format!("{line1}\n{line2}\n{line3}\n{line4}");
        let line2_start = line1.len() + "\n".len();
        let line3_start = line2_start + line2.len() + "\n".len();
        let line4_start = line3_start + line3.len() + "\n".len();
        assert_eq!(Location::char_idx_to_line_and_col(&source, 0), (0, 0));
        assert_eq!(
            Location::char_idx_to_line_and_col(&source, line2_start),
            (1, 0)
        );
        assert_eq!(
            Location::char_idx_to_line_and_col(&source, line3_start),
            (2, 0)
        );
        assert_eq!(
            Location::char_idx_to_line_and_col(&source, line4_start),
            (3, 0)
        );
        assert_eq!(
            Location::char_idx_to_line_and_col(&source, line4_start + 2),
            (3, 2)
        );
        assert_eq!(
            Location::char_idx_to_line_and_col(&source, line2_start + 4),
            (1, 4)
        );
    }

    #[test]
    fn processed_char_idx_to_raw_char_idx() -> Result<(), Box<dyn std::error::Error>> {
        let prelude = "a = 1;\n";
        let block = r#"<? printf("very_long_identifier = 2;"); ?>"#;
        let epilogue = "\nc = 3;";
        let expected_block = "very_long_identifier = 2;";
        let source = format!("{prelude}{block}{epilogue}");
        let file = File::from_str(&source);
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, 0)?,
            Position::Normal(0)
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, prelude.len() - 1)?,
            Position::Normal(prelude.len() - 1)
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(&file, prelude.len())?,
            Position::InPreProcessorBlock {
                pp_block: prelude.len()..prelude.len() + block.len()
            }
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(
                &file,
                prelude.len() + expected_block.len() - 1
            )?,
            Position::InPreProcessorBlock {
                pp_block: prelude.len()..prelude.len() + block.len()
            }
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(
                &file,
                prelude.len() + expected_block.len()
            )?,
            Position::Normal(prelude.len() + block.len())
        );
        assert_eq!(
            Location::processed_char_idx_to_raw_char_idx(
                &file,
                prelude.len() + expected_block.len() + 1
            )?,
            Position::Normal(prelude.len() + block.len() + 1)
        );
        Ok(())
    }

    #[test]
    fn to_processed_char_range() {
        let prelude = "a = 1;\n";
        let block = r#"<? printf("identifier = 2;"); ?>"#;
        let epilogue = "\nc = 3;";
        let expected_block = "identifier = 2;";
        let source = format!("{prelude}{block}{epilogue}");
        let file = Rc::new(File::from_str(&source));
        let location = Location {
            section: None,
            file: file.clone(),
            line_col: LineCol {
                start_line: 0,
                start_column: 1,
                end_line: 2,
                end_column: epilogue.len(),
            },
        };
        let range = location.to_processed_char_range();
        let expected_end = prelude.len() + expected_block.len() + epilogue.len();
        assert_eq!(range.start, 0);
        assert_eq!(range.end, expected_end);

        let location = Location {
            section: None,
            file,
            line_col: LineCol {
                start_line: 1,
                start_column: 3,
                end_line: 1,
                end_column: 4,
            },
        };
        let range = location.to_processed_char_range();
        let expected_start = prelude.len() + 2;
        assert_eq!(range.start, expected_start);
        assert_eq!(range.end, expected_start + 1);
    }
}
