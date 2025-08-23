use crate::iterators::lines_range::LineRanges;
/// Iterates over lines in a string, returning each line and its full length (including line endings).
/// The line content excludes the line ending, but the length includes it.
pub struct Lines<'a> {
    string: &'a str,
    it: LineRanges<'a>,
}

impl<'a> Lines<'a> {
    pub fn new(string: &'a str) -> Self {
        Self {
            string,
            it: LineRanges::new(string),
        }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = (&'a str, usize);
    fn next(&mut self) -> Option<Self::Item> {
        self.it.next().map(|(range, len)| {
            let ret = &self.string[range];
            (ret, len)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::iterators::lines::Lines;

    #[test]
    fn normal() {
        let source = "a\nb\nc";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ("a", 2));
        assert_eq!(lines[1], ("b", 2));
        assert_eq!(lines[2], ("c", 1));
    }

    #[test]
    fn empty_string() {
        let source = "";
        assert_eq!(Lines::new(source).count(), 0);
    }

    #[test]
    fn no_newline() {
        let source = "a";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], ("a", 1));
    }

    #[test]
    fn newline_at_end() {
        let source = "a\n";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], ("a", 2));
    }

    #[test]
    fn newline_at_start() {
        let source = "\na";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], ("", 1));
        assert_eq!(lines[1], ("a", 1));
    }

    #[test]
    fn newline_at_start_and_end() {
        let source = "\na\n";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], ("", 1));
        assert_eq!(lines[1], ("a", 2));
    }

    #[test]
    fn windows_newline() {
        let source = "a\r\nb\r\nc";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ("a", 3));
        assert_eq!(lines[1], ("b", 3));
        assert_eq!(lines[2], ("c", 1));
    }

    #[test]
    fn windows_newline_at_end() {
        let source = "a\r\n";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], ("a", 3));
    }

    #[test]
    fn windows_newline_at_start() {
        let source = "\r\na";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], ("", 2));
        assert_eq!(lines[1], ("a", 1));
    }

    #[test]
    fn windows_newline_at_start_and_end() {
        let source = "\r\na\r\n";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], ("", 2));
        assert_eq!(lines[1], ("a", 3));
    }

    #[test]
    fn mixed_newline() {
        let source = "a\nb\r\nc";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ("a", 2));
        assert_eq!(lines[1], ("b", 3));
        assert_eq!(lines[2], ("c", 1));
    }

    #[test]
    fn empty_line() {
        let source = "a\n\nb";
        let lines = Lines::new(source).collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], ("a", 2));
        assert_eq!(lines[1], ("", 1));
        assert_eq!(lines[2], ("b", 1));
    }
}
