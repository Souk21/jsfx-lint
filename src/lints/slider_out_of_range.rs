use crate::location::Location;
use crate::rcsubstring::RcSubString;
use crate::value::Value;
use crate::{
    IssueKind, Program, access,
    access::{Info, TopLevel},
    issue::IssueTracker,
    meta::Meta,
    variables::MaybeBoundToSlider,
};

/// Report out-of-range slider assignment
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for variable in program.scope.variables.values() {
        let MaybeBoundToSlider::Some(Meta::Slider {
            min,
            max,
            identifier,
            id,
            ..
        }) = crate::variables::is_bound_to_slider(&program.metas, variable.name.as_str())
        else {
            continue;
        };
        for access in &variable.accesses {
            let TopLevel {
                info:
                    Info {
                        kind: access::Kind::Write { value, .. },
                        location,
                        ..
                    },
                ..
            } = access
            else {
                continue;
            };
            let Value::Number(number) = value else {
                continue;
            };
            check_min(issues, identifier, id, location, *number, *min);
            check_max(issues, identifier, id, location, *number, *max);
        }
    }
}

fn check_max(
    issues: &mut IssueTracker,
    identifier: &Option<RcSubString>,
    id: &RcSubString,
    location: &Location,
    number: f64,
    max: f64,
) {
    if number <= max {
        return;
    }
    let text = identifier.as_ref().map_or_else(
        || format!("slider{id}"),
        |identifier| format!("{identifier} (slider{id})"),
    );
    issues.add(
        IssueKind::SliderOutOfRange,
        location,
        format!("Setting {text} to a value above its maximum (maximum is {max}, got {number})"),
    );
}

fn check_min(
    issues: &mut IssueTracker,
    identifier: &Option<RcSubString>,
    id: &RcSubString,
    location: &Location,
    number: f64,
    min: f64,
) {
    if number >= min {
        return;
    }
    let text = identifier.as_ref().map_or_else(
        || format!("slider{id}"),
        |identifier| format!("{identifier} (slider{id})"),
    );
    issues.add(
        IssueKind::SliderOutOfRange,
        location,
        format!("Setting {text} to a value below its minimum (minimum is {min}, got {number})"),
    );
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn slider_n_above_max() {
        let source = indoc! {"
            slider1:0<0,1,1>Slider1
            @init
            slider1 = 2;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderOutOfRange));
    }
    #[test]
    fn slider_n_below_min() {
        let source = indoc! {"
            slider1:0<0,1,1>Slider1
            @init
            slider1 = -2;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderOutOfRange));
    }
    #[test]
    fn slider_id_above_max() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Slider1
            @init
            foo = 2;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderOutOfRange));
    }
    #[test]
    fn slider_id_below_min() {
        let source = indoc! {"
            slider1:foo=0<0,1,1>Slider1
            @init
            foo = -2;
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderOutOfRange));
    }
}
