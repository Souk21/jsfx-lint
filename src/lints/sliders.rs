use crate::variables::looks_like_slider_n_var;
use crate::{
    IssueKind, MAX_SLIDER_COUNT, Program, issue::IssueTracker, meta::Meta,
    variables::MaybeSliderNVar,
};

pub fn lint(program: &Program, issues: &mut IssueTracker) {
    warn_sliders(program, issues);
    warn_variable_slider_lookalikes(program, issues);
}

fn warn_sliders(program: &Program, issues: &mut IssueTracker) {
    // Check for declared sliders that are not read/written to
    warn_slider_not_accessed(program, issues);
    // Check access to non-existing sliders
    warn_accessing_non_existing_slider(program, issues);
    warn_slider_labels(&program.metas, issues);
}

fn warn_slider_labels(metas: &[Meta], issues: &mut IssueTracker) {
    // If those slider params are wrong, envelopes/automations have issues with the parameter
    for meta in metas {
        if let Meta::Slider {
            min,
            max,
            step,
            labels: Some(labels),
            id,
            location,
            ..
        } = meta
        {
            // Min should be 0
            if !approx(*min, 0.0) {
                issues.add(
                    IssueKind::SliderLabels,
                    location,
                    format!("slider{id} minimum should be 0, as it has labels. (currently {min})"),
                );
            }
            // Step should be 1 or 0
            if !approx(*step, 1.0) && !approx(*step, 0.0) {
                issues.add(
                    IssueKind::SliderLabels,
                    location,
                    format!("slider{id} step should be 1, as it has labels. (currently {step})"),
                );
            }
            // Slider max should be labels.len() - 1
            if !approx_u(*max, labels.len() - 1) {
                let s = if labels.len() > 1 { "s" } else { "" };
                issues.add(
                    IssueKind::SliderLabels,
                    location,
                    format!(
                        "slider{id} maximum should be {}, as it has {} label{s}. (currently {max})",
                        labels.len() - 1,
                        labels.len()
                    ),
                );
            }
        }
    }
}

fn warn_accessing_non_existing_slider(program: &Program, issues: &mut IssueTracker) {
    for (_, variable) in program.scope.variables.iter().filter(|(key, _)| {
        matches!(
            looks_like_slider_n_var(&program.metas, key),
            MaybeSliderNVar::NonExisting
        )
    }) {
        issues.add(
            IssueKind::UnknownSlider,
            variable.first_location(),
            format!("{} doesn't exist", variable.name),
        );
    }
}

fn warn_slider_not_accessed(program: &Program, issues: &mut IssueTracker) {
    // If `slider()` builtin is called anywhere in the program, there's no need to check for unused sliders
    // as this function can set/read slider with index only known at runtime
    let has_slider_calls = program.sections.values().any(|section| {
        section.fun_calls.iter().any(|fun_call| {
            fun_call
                .fun
                .as_ref()
                .is_some_and(|fun| fun.is_builtin && fun.name.as_str() == "slider")
        })
    });
    if has_slider_calls {
        return;
    }

    for meta in &program.metas {
        if let Meta::SliderPath { id, location, .. } | Meta::Slider { id, location, .. } = meta {
            let mut is_read = false;
            let mut is_written = false;
            let slider_with_id = format!("slider{id}");
            let slider_n_works = looks_like_slider_n_var(&program.metas, &slider_with_id);
            let slider_n_works = matches!(slider_n_works, MaybeSliderNVar::Some(_));
            if slider_n_works && let Some(variable) = program.scope.variables.get(&slider_with_id) {
                is_read = variable.is_read();
                is_written = variable.is_written();
            }

            let mut slider_identifier_fmt = String::new();
            if !is_read && !is_written {
                // Look for the slider identifier if any
                if let Meta::Slider {
                    identifier: Some(identifier),
                    ..
                } = meta
                {
                    slider_identifier_fmt = format!("({identifier}) ");
                    program
                        .scope
                        .variables
                        .get(identifier.to_lower())
                        .inspect(|var| {
                            is_read = var.is_read();
                            is_written = var.is_written();
                        });
                }
            }

            if !is_read && !is_written {
                issues.add(
                    IssueKind::UnusedSlider,
                    location,
                    format!("{slider_with_id} {slider_identifier_fmt}is never used"),
                );
            }
        }
    }
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < f64::EPSILON
}

fn approx_u(a: f64, b: usize) -> bool {
    #[allow(clippy::cast_precision_loss)]
    let b = b as f64;
    approx(a, b)
}

fn warn_variable_slider_lookalikes(program: &Program, issues: &mut IssueTracker) {
    for (key, variable) in &program.scope.variables {
        let variable_name = &variable.name;
        match looks_like_slider_n_var(&program.metas, key) {
            MaybeSliderNVar::Shadowed(Meta::Slider {
                identifier: Some(identifier),
                ..
            }) => {
                issues.add(IssueKind::ShadowedSliderN,
                           variable.first_location(),
                           format!("{variable_name} looks like a slider variable, but isn't bound to the {identifier} slider. Slider that are bound to a variable can't be accessed using sliderN variables.")
                );
            }
            MaybeSliderNVar::LooksLike => {
                issues.add(IssueKind::LooksLikeSliderN,
                           variable.first_location(),
                           format!("{variable_name} looks like a slider variable, but sliders only go from slider1 to slider{MAX_SLIDER_COUNT}.")
                );
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::file::File;
    use crate::meta::Meta;
    use crate::{IssueKind, MAX_SLIDER_COUNT};

    #[test]
    fn unused_slider() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Foo
            @init
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedSlider));
    }
    #[test]
    fn unused_slider_with_n() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Foo
            @init
            slider1 = 1;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedSlider));
    }
    #[test]
    fn used_slider() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Foo
            @init
            foo = 1;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedSlider));
    }
    #[test]
    fn unused_slider_n() {
        let source = indoc! {"
            slider1:0<0,1,1>Foo
            @init
            0;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnusedSlider));
    }

    #[test]
    fn used_slider_n() {
        let source = indoc! {"
            slider1:0<0,1,1>Foo
            @init
            slider1 = 1;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedSlider));
    }

    #[test]
    fn unknown_slider() {
        let source = indoc! {"
            slider1:0<0,1,1>Foo
            @init
            slider2 = 10;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::UnknownSlider));
    }

    #[test]
    fn slider_labels_ok() {
        let source = indoc! {"
            slider1:0<0, 1, 1{On,Off}>Foo
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SliderLabels));
    }

    #[test]
    fn slider_labels() {
        let source = indoc! {"
            slider1:0<0,5,1{zerolabel,onelabel,twolabel,threelabel,fourlabel,fivelabel}>some setting
        "};
        let (program, _) = File::lint_with_default_config(source);
        assert!(matches!(
            &program.metas[0],
            Meta::Slider {
                labels: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn slider_labels_len() {
        let source = indoc! {"
            slider1:0<0, 2, 1{On,Off}>Foo
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderLabels));
    }

    #[test]
    fn slider_labels_min() {
        let source = indoc! {"
            slider1:0<1, 2, 1{On,Off,Both}>Foo
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderLabels));
    }

    #[test]
    fn slider_labels_step() {
        let source = indoc! {"
            slider1:0<0, 2, 2{On,Off,Both}>Foo
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderLabels));
    }

    #[test]
    fn shadowed_slider_n() {
        let source = indoc! {"
            slider1:foo=0<0, 2, 1{On,Off}>Foo
            @init
            slider1 = 10;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ShadowedSliderN));
    }

    #[test]
    fn looks_like_slider_n() {
        let source = format!("@init\nslider{} = 10;", MAX_SLIDER_COUNT + 1);
        let (_, issues) = File::lint_with_default_config(source.as_str());
        assert!(issues.has(&IssueKind::LooksLikeSliderN));
    }

    #[test]
    fn slider_is_called() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Foo
            @init
            slider(1);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::UnusedSlider));
    }
    #[test]
    fn var_starts_with_slider() {
        let source = indoc! {"
            @init
            sliderfft = 10;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::LooksLikeSliderN));
    }
}
