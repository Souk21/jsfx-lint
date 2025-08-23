use std::str::CharIndices;

/// An iterator that returns the positions of '.' characters in a string
pub struct Dots<'a> {
    char_indices: CharIndices<'a>,
}
impl<'a> Dots<'a> {
    pub fn new(string: &'a str) -> Self {
        Self {
            char_indices: string.char_indices(),
        }
    }
}
impl Iterator for Dots<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.char_indices.next()?;
        if next.1 == '.' {
            Some(next.0)
        } else {
            self.next()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dots() {
        let source = "a.b.c..d";
        let dots: Vec<_> = Dots::new(source).collect();
        assert_eq!(dots, vec![1, 3, 5, 6]);
    }

    #[test]
    fn test_no_dots() {
        let source = "abc";
        assert!(Dots::new(source).next().is_none());
    }

    #[test]
    fn test_empty_string() {
        let source = "";
        assert!(Dots::new(source).next().is_none());
    }

    #[test]
    fn test_dots_at_end() {
        let source = "abc.";
        let dots: Vec<_> = Dots::new(source).collect();
        assert_eq!(dots, vec![3]);
    }
    #[test]
    fn test_dots_at_start() {
        let source = ".abc";
        let dots: Vec<_> = Dots::new(source).collect();
        assert_eq!(dots, vec![0]);
    }
    #[test]
    fn test_dots_only() {
        let source = "...";
        let dots: Vec<_> = Dots::new(source).collect();
        assert_eq!(dots, vec![0, 1, 2]);
    }
}
