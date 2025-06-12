use crate::{IssueKind, issue::IssueTracker, meta::Meta, program::Program};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for meta in &program.metas {
        let Meta::Slider {
            id, desc, location, ..
        } = &meta
        else {
            continue;
        };
        if desc.is_empty() {
            issues.add(
                IssueKind::SliderWithoutDescription,
                location,
                format!("slider{id} has no description. It will not be visible in the default UI and will not be automatable."),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{IssueKind, file::File};

    #[test]
    fn slider_without_description() {
        let source = "slider1:volume=0<0,1,1>";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderWithoutDescription));
    }

    #[test]
    fn slider_with_description() {
        let source = "slider1:volume=0<0,1,1>Volume";
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SliderWithoutDescription));
    }
}
