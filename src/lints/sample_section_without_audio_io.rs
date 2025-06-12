use crate::{
    IssueKind,
    issue::IssueTracker,
    location::{LineCol, Location},
    program::Program,
    variables::IsBuiltin,
};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    let Some(sample) = program.sections.get("sample") else {
        return;
    };

    if sample.fun_calls.iter().any(|fun_call| {
        fun_call.fun.as_ref().is_some_and(|called_fun| {
            let name = called_fun.name.as_str();
            called_fun.is_builtin && (name == "slider_next_chg" || name == "spl")
        })
    }) {
        return;
    }

    for (name, variable) in &program.scope.variables {
        if matches!(
            program.scope.is_builtin(name, &program.metas),
            IsBuiltin::None
        ) {
            continue;
        }
        for access in &variable.accesses {
            if access.section == "sample"
                && access
                    .info
                    .accessed_as
                    .to_ascii_lowercase()
                    .starts_with("spl")
            {
                return;
            }
        }
    }
    let chunk = sample
        .chunks
        .first()
        .expect("Sample section should have at least one chunk");
    let location = Location {
        file: chunk.file.clone(),
        section: Some(sample.kind),
        line_col: LineCol {
            start_line: chunk.line_pos,
            start_column: 1,
            end_line: chunk.line_pos,
            end_column: "@sample".len() + 1,
        },
    };
    issues.add(IssueKind::SampleSectionWithoutAudioIo, &location, "@sample section doesn't contain any audio input/output operations. This kind of work should be done in @block".to_string());
}

#[cfg(test)]
mod tests {
    use crate::{IssueKind, file::File};
    use indoc::indoc;

    #[test]
    pub fn sample_section_without_audio_io() {
        let source = indoc! {"
            @sample
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SampleSectionWithoutAudioIo));
    }

    #[test]
    pub fn sample_section_with_slider_next_chg() {
        let source = indoc! {"
            @sample
            slider_next_chg(0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SampleSectionWithoutAudioIo));
    }

    #[test]
    pub fn sample_section_with_audio_io() {
        let source = indoc! {"
            @sample
            spl1 = 0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SampleSectionWithoutAudioIo));
    }

    #[test]
    pub fn sample_section_with_audio_io_fn() {
        let source = indoc! {"
            @sample
            spl(1) = 0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SampleSectionWithoutAudioIo));
    }

    #[test]
    pub fn sample_section_with_audio_io_read() {
        let source = indoc! {"
            @sample
            a = spl1;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SampleSectionWithoutAudioIo));
    }

    #[test]
    pub fn sample_section_with_audio_io_fn_read() {
        let source = indoc! {"
            @sample
            a = spl(1);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SampleSectionWithoutAudioIo));
    }
}
