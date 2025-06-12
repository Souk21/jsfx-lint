use crate::issue_kind_to_string;
use crate::{IssueKind, Severity, get_default_config, location::Location};
use colored::Colorize;
use std::io::Write;
use std::{collections::HashMap, io::BufWriter};

pub struct Issue {
    pub location: Location,
    pub text: String,
    pub kind: IssueKind,
    pub severity: Severity,
}

pub struct IssueTracker {
    issues: Vec<Issue>,
    config: HashMap<IssueKind, Severity>,
}

impl IssueTracker {
    pub const fn new_with_config(config: HashMap<IssueKind, Severity>) -> Self {
        Self {
            issues: Vec::new(),
            config,
        }
    }

    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            config: get_default_config(),
        }
    }

    pub fn has(&self, kind: &IssueKind) -> bool {
        self.issues.iter().any(|issue| issue.kind == *kind)
    }

    pub fn count(&self, kind: &IssueKind) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.kind == *kind)
            .count()
    }

    pub fn add(&mut self, issue_kind: IssueKind, location: &Location, text: String) {
        let severity = self
            .config
            .get(&issue_kind)
            .expect("Unregistered issue kind")
            .clone();
        self.issues.push(Issue {
            location: location.clone(),
            severity,
            text,
            kind: issue_kind,
        });
    }

    pub fn sort(&mut self) {
        self.issues.sort_by(|a, b| {
            if a.severity != b.severity {
                return a.severity.cmp(&b.severity);
            }
            a.location.compare(&b.location)
        });
    }

    pub fn report(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.sort();
        let mut writer = BufWriter::new(std::io::stdout());
        for Issue {
            location,
            text,
            severity,
            kind,
        } in &self.issues
        {
            let color_fn = match severity {
                Severity::Warning => <&str as Colorize>::yellow,
                Severity::Error => <&str as Colorize>::red,
                Severity::Style => <&str as Colorize>::blue,
                Severity::Silent => continue,
            };
            let severity_str = match severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Style => "style",
                Severity::Silent => unreachable!(),
            };

            let printable = location.to_printable()?;
            if printable.in_preprocessor && severity != &Severity::Error {
                // Skip preprocessor style issues and warnings
                continue;
            }

            write!(writer, "{}: ", color_fn(severity_str).bold())?;
            writeln!(writer, "{}", text.bold())?;
            let line_idx = if printable.in_preprocessor {
                String::new()
            } else {
                // Lines are displayed 1-based
                (printable.shown_line_col.start_line + 1).to_string()
            };
            let line_col = &printable.shown_line_col;
            let left_padding_size = line_idx.len() + 1;
            let left_padding = " ".repeat(left_padding_size);
            let arrow_padding = " ".repeat(left_padding_size.saturating_sub(1));
            let point_at = line_col.start_column;
            let mut pointer_padding = " ".repeat(point_at.saturating_sub(1));
            let pointer_end = line_col.end_column;
            let mut pointer_len = pointer_end.saturating_sub(point_at);
            let arrow = "-->".blue().bold();
            let kind_str = issue_kind_to_string(kind);
            writeln!(
                writer,
                "{arrow_padding}{arrow} {} ({})",
                printable.location_text,
                kind_str.italic()
            )?;
            let mut line = printable.line;
            let max_len = 120;
            if line.len() > max_len {
                let start = line_col.start_column.saturating_sub(max_len / 2);
                let mut end = line_col.start_column.saturating_add(max_len / 2);
                end = end.clamp(start, line.len());
                line = &line[start..end];
                pointer_padding = " ".repeat(point_at.saturating_sub(1 + start));
                let pointer_end = line_col.end_column.clamp(start, end);
                pointer_len = pointer_end.saturating_sub(point_at);
            }
            let pointer = color_fn(&"^".repeat(pointer_len)).bold();
            let separator = "|".blue().bold();
            writeln!(writer, "{left_padding}{separator}")?;
            writeln!(writer, "{line_idx} {separator} {line}")?;
            writeln!(
                writer,
                "{left_padding}{separator} {pointer_padding}{pointer}"
            )?;
            // New line for readability
            writeln!(writer)?;
        }
        Ok(())
    }
}

impl Default for IssueTracker {
    fn default() -> Self {
        Self::new()
    }
}
