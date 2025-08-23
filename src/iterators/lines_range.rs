use std::{ops::Range, str::CharIndices};

/// Iterates over lines in a string, returning each line and its full length (including line endings).
/// The line content excludes the line ending, but the length includes it.
pub struct LineRanges<'a> {
    string: &'a str,
    byte_pos: usize,
    char_indices: CharIndices<'a>,
}

impl<'a> LineRanges<'a> {
    pub fn new(string: &'a str) -> Self {
        Self {
            string,
            byte_pos: 0,
            char_indices: string.char_indices(),
        }
    }
}

impl Iterator for LineRanges<'_> {
    type Item = (Range<usize>, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let mut added_to_terminator = 0;
        if self.byte_pos >= self.string.len() {
            return None;
        }
        for (idx, char) in self.char_indices.by_ref() {
            if char == '\n' {
                let range = self.byte_pos..idx - added_to_terminator;
                // +1 for the newline character
                let len = range.len() + added_to_terminator + 1;
                self.byte_pos = idx + 1;
                return Some((range, len));
            }
            if char == '\r' {
                added_to_terminator += 1;
                continue;
            }
        }
        let range = self.byte_pos..self.string.len();
        let len = range.len();
        self.byte_pos = self.string.len();
        Some((range, len))
    }
}
