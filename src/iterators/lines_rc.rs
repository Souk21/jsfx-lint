use crate::iterators::lines_range::LineRanges;
use crate::rcsubstring::RcSubString;

/// Iterates over lines in a string, returning each line and its full length (including line endings).
/// The line content excludes the line ending, but the length includes it.
pub struct LinesRc<'a> {
    string: &'a RcSubString,
    it: LineRanges<'a>,
}

impl<'a> LinesRc<'a> {
    pub fn new(string: &'a RcSubString) -> Self {
        Self {
            string,
            it: LineRanges::new(string.as_str()),
        }
    }
}

impl Iterator for LinesRc<'_> {
    type Item = (RcSubString, usize);
    fn next(&mut self) -> Option<Self::Item> {
        self.it.next().map(|(range, len)| {
            let ret = self.string.substr(range);
            (ret, len)
        })
    }
}
