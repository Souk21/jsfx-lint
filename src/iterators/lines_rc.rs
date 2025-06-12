use crate::iterators::lines_range::LineRanges;
use crate::rcsubstring::RcSubString;

/// Iterator returning each line and the length of the line including the line ending
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
