use crate::iterators::lines::Lines;
use std::cell::OnceCell;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::{Deref, Range};
use std::rc::Rc;
use std::{cmp::PartialEq, ops::RangeBounds};

/// A reference-counted substring.
#[derive(Clone, Eq)]
pub struct RcSubString {
    string: Option<Rc<str>>,
    range: Range<usize>,
    lowercase_cache: OnceCell<String>,
}

impl Default for RcSubString {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Debug for RcSubString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl Hash for RcSubString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.deref().hash(state);
    }
}

impl Display for RcSubString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self)
    }
}

impl PartialEq for RcSubString {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Deref for RcSubString {
    type Target = str;
    fn deref(&self) -> &str {
        self.string
            .as_ref()
            .map_or("", |s| s.as_ref())
            .get(self.range.clone())
            .expect("Invalid range in RcSubString")
    }
}

impl RcSubString {
    pub const fn empty() -> Self {
        Self {
            string: None,
            range: 0..0,
            lowercase_cache: OnceCell::new(),
        }
    }

    pub fn as_str(&self) -> &str {
        self
    }

    pub fn from_str(string: &str) -> Self {
        Self {
            string: Some(Rc::from(string)),
            range: 0..string.len(),
            lowercase_cache: OnceCell::new(),
        }
    }

    /// Returns the entire line and the offset of `pos` in that line
    pub fn line_at_pos(&self, mut pos: usize) -> Option<(&str, usize)> {
        pos += self.range.start;
        let source = self.string.as_ref()?;
        let mut line_start = 0;
        for (line, line_len) in Lines::new(source) {
            let line_end = line_start + line_len;
            if pos <= line_end {
                return Some((line, pos - line_start));
            }
            line_start = line_end + 1;
        }
        None
    }

    /// Returns the index of the line (1-based) at `pos` and the offset of `pos` in that line (0-based)
    pub fn line_pos_at_pos(&self, pos: usize) -> Option<(usize, usize)> {
        let pos = pos + self.range.start;
        let source = self.string.as_ref()?.as_ref();
        let mut line_start = 0;
        for (idx, (_, line_len)) in Lines::new(source).enumerate() {
            let line_end = line_start + line_len;
            if pos < line_end {
                return Some((idx + 1, pos - line_start));
            }
            line_start = line_end;
        }
        None
    }

    pub fn substr<T: RangeBounds<usize>>(&self, range: T) -> Self {
        let range_start = match range.start_bound() {
            std::ops::Bound::Included(i) => i + self.range.start,
            std::ops::Bound::Unbounded => self.range.start,
            std::ops::Bound::Excluded(_) => unreachable!(),
        };
        let range_end = match range.end_bound() {
            std::ops::Bound::Included(i) => self.range.start + i + 1,
            std::ops::Bound::Excluded(i) => self.range.start + i,
            std::ops::Bound::Unbounded => self.range.end,
        };
        let range = range_start.clamp(self.range.start, self.range.end)
            ..range_end.clamp(self.range.start, self.range.end);
        Self {
            string: self.string.clone(),
            range,
            lowercase_cache: OnceCell::new(),
        }
    }

    pub const fn len(&self) -> usize {
        self.range.end - self.range.start
    }

    pub fn char_at(&self, n: usize) -> Option<char> {
        self.chars().nth(self.range.start + n)
    }

    pub fn contains_at_pos(&self, string: &str, pos: usize) -> bool {
        let len = string.len();
        let start = pos;
        let end = start + len;
        if start >= self.len() || end > self.len() {
            return false;
        }
        let substr = self.substr(start..end);
        &*substr == string
    }

    pub fn to_lower(&self) -> &str {
        let lowercase = self
            .lowercase_cache
            .get_or_init(|| self.to_string().to_ascii_lowercase());
        lowercase.as_str()
    }

    pub fn eq_ignore_ascii_case(&self, other: &str) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.to_lower() == other.to_ascii_lowercase()
    }

    /// Strip prefix regardless of case, return a `RcSubString` with original casing
    pub fn strip_prefix_case(&self, prefix: &str) -> Option<Self> {
        if self.to_lower().starts_with(&prefix.to_ascii_lowercase()) {
            return Some(self.substr(prefix.len()..self.len()));
        }
        None
    }

    pub fn strip_prefix(&self, prefix: &str) -> Option<Self> {
        if self.as_str().starts_with(prefix) {
            return Some(self.substr(prefix.len()..self.len()));
        }
        None
    }

    pub fn trim(&self) -> Self {
        if self.string.is_none() {
            return Self::empty();
        }
        for (start, c) in self.char_indices() {
            if !c.is_whitespace() {
                for (end, c) in self.char_indices().rev() {
                    if !c.is_whitespace() {
                        return self.substr(start..end + c.len_utf8());
                    }
                }
            }
        }
        Self::empty()
    }

    /// If the identifier starts with `prefix` and a dot, return the suffix with the dot.
    /// Example:
    /// ```
    ///  "this" => ""
    ///  "this.a" => ".a"
    ///  "ref.hello" = ".hello"
    /// ```
    pub fn extract_suffix(&self, prefix: &str) -> Option<Self> {
        if self.to_lower() == prefix.to_ascii_lowercase() {
            return Some(Self::empty());
        }
        self.strip_prefix_case(prefix).and_then(|suffix| {
            if suffix.as_str().starts_with('.') {
                Some(suffix)
            } else {
                None
            }
        })
    }
}
