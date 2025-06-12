use std::rc::Rc;

use regex::{Captures, Regex};

use crate::iterators::lines_rc::LinesRc;
use crate::{
    IssueKind, MAX_SLIDER_COUNT,
    file::File,
    issue::IssueTracker,
    location::{LineCol, Location},
    rcsubstring::RcSubString,
};

#[derive(Debug)]
pub enum Meta {
    Desc(RcSubString),
    InPin(RcSubString),
    OutPin(RcSubString),
    Filename(RcSubString),
    Option(RcSubString),
    Import {
        path: RcSubString,
        location: Location,
    },
    Slider {
        id: RcSubString,
        identifier: Option<RcSubString>,
        default_str: RcSubString,
        default: f64,
        min_str: Option<RcSubString>,
        min: f64,
        max_str: Option<RcSubString>,
        max: f64,
        step_str: Option<RcSubString>,
        step: f64,
        desc: RcSubString,
        labels: Option<Vec<RcSubString>>,
        location: Location,
    },
    SliderPath {
        id: RcSubString,
        path: RcSubString,
        default: RcSubString,
        desc: RcSubString,
        location: Location,
    },
    Generic(RcSubString, RcSubString),
}

pub fn parse_metas(
    source: &RcSubString,
    file: &Rc<File>,
    first_section_pos: Option<usize>,
    issues: &mut IssueTracker,
) -> Vec<Meta> {
    let mut metas = Vec::new();
    // If no sections were found, the entire source is metas
    let first_section = first_section_pos.unwrap_or_else(|| source.len());
    let meta_source = source.substr(0..first_section);
    let metas_regex = Regex::new(r"^(\w+):(.+)$").unwrap();

    for (line_idx, (line, _)) in LinesRc::new(&meta_source).enumerate() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with("import ") && line.len() >= "import x".len() {
            parse_import(file, &mut metas, line_idx, &line);
            continue;
        }
        if let Some(captures) = metas_regex.captures(line.as_str()) {
            parse_other(&line, file, issues, &mut metas, &captures, line_idx);
            continue;
        }
    }
    metas
}

fn parse_other(
    line: &RcSubString,
    file: &Rc<File>,
    issues: &mut IssueTracker,
    metas: &mut Vec<Meta>,
    captures: &Captures,
    line_pos: usize,
) {
    let Some(meta_type) = captures.get(1) else {
        return;
    };
    let Some(meta_value) = captures.get(2) else {
        return;
    };
    let meta_type = line.substr(meta_type.start()..meta_type.end());
    let meta_value = line.substr(meta_value.start()..);
    let location = Location {
        section: None,
        file: file.clone(),
        line_col: LineCol {
            start_line: line_pos,
            end_line: line_pos,
            start_column: 1,
            end_column: meta_type.len() + ":".len() + meta_value.len() + 1,
        },
    };
    if let Some(id) = meta_type.strip_prefix("slider") {
        parse_any_slider(id, &meta_value, issues, &location, metas);
        return;
    }
    match meta_type.as_str() {
        "options" => Regex::new(r"\S+\s*=\s*\S+|\S+")
            .unwrap()
            .find_iter(meta_value.as_str())
            .for_each(|m| {
                metas.push(Meta::Option(meta_value.substr(m.start()..m.end())));
            }),
        "desc" => metas.push(Meta::Desc(meta_value)),
        "in_pin" => metas.push(Meta::InPin(meta_value)),
        "out_pin" => metas.push(Meta::OutPin(meta_value)),
        "filename" => metas.push(Meta::Filename(meta_value)),
        _ => metas.push(Meta::Generic(meta_type, meta_value)),
    }
}

fn parse_import(file: &Rc<File>, metas: &mut Vec<Meta>, line_idx: usize, line: &RcSubString) {
    if let Some(path) = line.strip_prefix("import ") {
        let location = Location {
            section: None,
            file: file.clone(),
            line_col: LineCol {
                start_line: line_idx,
                end_line: line_idx,
                start_column: 1,
                end_column: line.len() + 1,
            },
        };
        metas.push(Meta::Import { path, location });
    }
}

fn parse_error(location: &Location, issues: &mut IssueTracker) {
    issues.add(
        IssueKind::SliderParser,
        location,
        "Invalid slider syntax".into(),
    );
}

fn parse_any_slider(
    id: RcSubString,
    meta_value: &RcSubString,
    issues: &mut IssueTracker,
    location: &Location,
    metas: &mut Vec<Meta>,
) {
    let Ok(id_number) = id.parse::<usize>() else {
        // Invalid id
        return;
    };

    if id_number > MAX_SLIDER_COUNT {
        issues.add(
            IssueKind::SliderOverMaxId,
            location,
            format!("Max slider id is {MAX_SLIDER_COUNT}, got slider{id}"),
        );
        return;
    }

    if meta_value.trim().starts_with('/') {
        parse_slider_path(id, meta_value, location, metas, issues);
    } else {
        parse_slider(id, meta_value, metas, location, issues);
    }
}

fn parse_slider_path(
    id: RcSubString,
    meta_value: &RcSubString,
    location: &Location,
    metas: &mut Vec<Meta>,
    issues: &mut IssueTracker,
) {
    let first_colon = meta_value.find(':');
    let second_colon = first_colon.and_then(|comma| {
        meta_value[comma + ":".len()..]
            .find(':')
            .map(|second| comma + ":".len() + second)
    });

    let (Some(first_colon), Some(second_colon)) = (first_colon, second_colon) else {
        parse_error(location, issues);
        return;
    };

    let path = meta_value.substr(..first_colon).trim();
    let default = meta_value
        .substr(first_colon + ":".len()..second_colon)
        .trim();
    let desc = meta_value.substr(second_colon + ":".len()..).trim();

    metas.push(Meta::SliderPath {
        id,
        path,
        default,
        desc,
        location: location.clone(),
    });
}

fn parse_slider(
    id: RcSubString,
    meta_value: &RcSubString,
    metas: &mut Vec<Meta>,
    location: &Location,
    issues: &mut IssueTracker,
) {
    let opening_bracket_pos = meta_value.find('<');
    let closing_bracket_pos = opening_bracket_pos.and_then(|open| {
        meta_value[open + "<".len()..]
            .find('>')
            .map(|close| open + "<".len() + close)
    });
    let (Some(opening_bracket_pos), Some(closing_bracket_pos)) =
        (opening_bracket_pos, closing_bracket_pos)
    else {
        if opening_bracket_pos.is_some() {
            parse_error(location, issues);
            return;
        }
        parse_bracketless_slider(id, meta_value, location, metas, issues);
        return;
    };

    let identifier_and_default = meta_value.substr(..opening_bracket_pos);
    let identifier;
    let default_str;

    if let Some(equal_pos) = identifier_and_default.find('=') {
        (identifier, default_str) =
            get_identifier_and_default_str(&identifier_and_default, equal_pos, issues, location);
    } else {
        // No '=' found, there's no identifier and just a default value
        identifier = None;
        default_str = identifier_and_default.trim();
    }

    let default = find_number(default_str.as_str()).unwrap_or_else(|| {
        report_number_issue(issues, location, default_str.as_str(), "default");
        0.0
    });

    let params = meta_value.substr(opening_bracket_pos + "<".len()..closing_bracket_pos);
    let first_comma = params.find(',');
    let second_comma = first_comma.and_then(|comma| {
        params[comma + ",".len()..]
            .find(',')
            .map(|second| comma + ",".len() + second)
    });

    let min_str = first_comma.map_or_else(
        || {
            let p = params.trim();
            if p.is_empty() { None } else { Some(p) }
        },
        |comma| Some(params.substr(..comma).trim()),
    );

    let min = min_str.as_ref().map_or(0.0, |min_str| {
        find_number(min_str.as_str()).unwrap_or_else(|| {
            report_number_issue(issues, location, min_str.as_str(), "minimum");
            0.0
        })
    });

    let max_str = first_comma.map(|first_comma| {
        second_comma.map_or_else(
            || params.substr(first_comma + ",".len()..).trim(),
            |second_comma| params.substr(first_comma + ",".len()..second_comma).trim(),
        )
    });
    let max = max_str.as_ref().map_or(0.0, |param| {
        find_number(param.as_str()).unwrap_or_else(|| {
            report_number_issue(issues, location, param.as_str(), "maximum");
            0.0
        })
    });

    let step_str =
        second_comma.map(|second_comma| params.substr(second_comma + ",".len()..).trim());

    let labels = step_str.as_ref().and_then(|step_str| {
        let brace = step_str.find('{');
        brace.map(|brace| consume_labels(&step_str.substr(brace + "{".len()..)))
    });

    let step = step_str.as_ref().map_or(0.0, |step_str| {
        find_number(step_str.as_str()).unwrap_or_else(|| {
            // It's legal to only have labels and not a step
            if labels.is_some() {
                1.0
            } else {
                report_number_issue(issues, location, step_str.as_str(), "step");
                0.0
            }
        })
    });

    let desc = meta_value.substr(closing_bracket_pos + ">".len()..);

    metas.push(Meta::Slider {
        id,
        identifier,
        default_str,
        min_str,
        max_str,
        step_str,
        min,
        max,
        step,
        default,
        desc,
        labels,
        location: location.clone(),
    });
}

fn parse_bracketless_slider(
    id: RcSubString,
    meta_value: &RcSubString,
    location: &Location,
    metas: &mut Vec<Meta>,
    issues: &mut IssueTracker,
) {
    // Parsing this syntax "slider1:0,Sample Rate"
    let Some(comma) = meta_value.find(',') else {
        parse_error(location, issues);
        return;
    };
    let desc = meta_value.substr(comma + 1..);
    let default_str = meta_value.substr(..comma).trim();
    metas.push(Meta::Slider {
        id,
        identifier: None,
        default_str,
        min_str: None,
        max_str: None,
        step_str: None,
        min: 0f64,
        max: 0f64,
        step: 0f64,
        default: 0f64,
        desc,
        labels: None,
        location: location.clone(),
    });
}

fn get_identifier_and_default_str(
    identifier_and_default: &RcSubString,
    equal_pos: usize,
    issues: &mut IssueTracker,
    location: &Location,
) -> (Option<RcSubString>, RcSubString) {
    let before_equal = identifier_and_default.substr(..equal_pos).trim();
    if before_equal.is_empty() {
        issues.add(
            IssueKind::SliderInvalidIdentifier,
            location,
            "Empty identifier".into(),
        );
    } else {
        // before_equal is not empty, so it's safe to unwrap
        let first_char = before_equal
            .chars()
            .next()
            .expect("before_equal shouldn't be empty");
        let first_char_ok = first_char.is_alphabetic() || first_char == '_';
        let valid = first_char_ok
            && before_equal
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                .is_none();
        if !valid {
            issues.add(
                IssueKind::SliderInvalidIdentifier,
                location,
                format!("'{before_equal}' is not a valid EEL2 identifier"),
            );
        }
    }
    let identifier = Some(before_equal);
    let default_str = identifier_and_default
        .substr(equal_pos + "=".len()..)
        .trim();
    (identifier, default_str)
}

fn report_number_issue(
    issues: &mut IssueTracker,
    location: &Location,
    number: &str,
    word: &'static str,
) {
    issues.add(
        IssueKind::SliderParser,
        location,
        format!("expected a number for slider {word}, found: '{number}'. The {word} for this slider will be 0."),
    );
}

fn consume_labels(param: &RcSubString) -> Vec<RcSubString> {
    let end_brace = param.find('}');
    let labels = if let Some(end_brace) = end_brace {
        &param.substr(..end_brace)
    } else {
        param
    };
    split_labels(labels)
}

fn split_labels(labels: &RcSubString) -> Vec<RcSubString> {
    let mut labels_vec = Vec::new();
    let mut prev_comma_end = 0;
    while let Some(comma) = labels.as_str()[prev_comma_end..].find(',') {
        labels_vec.push(labels.substr(prev_comma_end..prev_comma_end + comma));
        prev_comma_end += comma + ",".len();
    }
    labels_vec.push(labels.substr(prev_comma_end..));
    labels_vec
}

fn find_number(text: &str) -> Option<f64> {
    // Find the longest string that parses as a number.
    // Number can be NaN or inf/-inf
    if let Ok(text_f64) = text.parse::<f64>() {
        // Full string is a number
        return Some(text_f64);
    }
    for (idx, _) in text.char_indices().rev() {
        if let Ok(text_f64) = text[..idx].parse::<f64>() {
            return Some(text_f64);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn basic() {
        let source = "slider1:foo=1.5<1, 10, 2>Bar";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "1");
        assert_eq!(identifier.as_ref().map(RcSubString::as_str), Some("foo"));
        assert_eq!(default_str.as_str(), "1.5");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "1");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "10");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "2");
        assert_eq!(desc.as_str(), "Bar");
        assert!(approx(*default, 1.5));
        assert!(approx(*min, 1.0));
        assert!(approx(*max, 10.0));
        assert!(approx(*step, 2.0));
        assert!(labels.is_none());
    }
    #[test]
    fn whitespace() {
        let source = "slider2:   hello=0<0,1,1>S";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "2");
        assert_eq!(identifier.as_ref().map(RcSubString::as_str), Some("hello"));
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "1");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "1");
        assert_eq!(desc.as_str(), "S");
        assert!(approx(*default, 0.0));
        assert!(approx(*min, 0.0));
        assert!(approx(*max, 1.0));
        assert!(approx(*step, 1.0));
        assert!(labels.is_none());
    }
    #[test]
    fn extra_chars() {
        let source = "slider13:2<1.1.2,10,0>min is 1.1";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "13");
        assert!(identifier.is_none());
        assert_eq!(default_str.as_str(), "2");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "1.1.2");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "10");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(desc.as_str(), "min is 1.1");
        assert!(approx(*default, 2.0));
        assert!(approx(*min, 1.1));
        assert!(approx(*max, 10.0));
        assert!(approx(*step, 0.0));
        assert!(labels.is_none());
    }
    #[test]
    fn notation() {
        let source = "slider14:2<0,1.12e3,0>max is 1120";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "14");
        assert!(identifier.is_none());
        assert_eq!(default_str.as_str(), "2");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "1.12e3");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(desc.as_str(), "max is 1120");
        assert!(approx(*default, 2.0));
        assert!(approx(*min, 0.0));
        assert!(approx(*max, 1120.0));
        assert!(approx(*step, 0.0));
        assert!(labels.is_none());
    }
    #[test]
    fn infinity() {
        let source = "slider19:inf<.1,10,0>default is infinity";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "19");
        assert!(identifier.is_none());
        assert_eq!(default_str.as_str(), "inf");
        assert_eq!(min_str.as_ref().unwrap().as_str(), ".1");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "10");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(desc.as_str(), "default is infinity");
        assert!(default.is_infinite());
        assert!(approx(*min, 0.1));
        assert!(approx(*max, 10.0));
        assert!(approx(*step, 0.0));
        assert!(labels.is_none());
    }
    #[test]
    fn labels() {
        let source = "slider24:0<1,10,3éäì   {On, Off}>Hi";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "24");
        assert!(identifier.is_none());
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "1");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "10");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "3éäì   {On, Off}");
        assert_eq!(desc.as_str(), "Hi");
        assert!(approx(*default, 0.0));
        assert!(approx(*min, 1.0));
        assert!(approx(*max, 10.0));
        assert!(approx(*step, 3.0));
        let Some(labels) = labels else {
            panic!("Expected labels");
        };
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].as_str(), "On");
        assert_eq!(labels[1].as_str(), " Off");
    }

    #[test]
    fn two_params() {
        let source = "slider24:foo=0<0,1>hey";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min_str,
            max_str,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "24");
        assert!(matches!(identifier, Some(identifier) if identifier.as_str() == "foo"));
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "1");
        assert_eq!(step_str, &None);
        assert_eq!(desc.as_str(), "hey");
        assert!(approx(*step, 0.0));
        assert!(labels.is_none());
    }

    #[test]
    fn one_param() {
        let source = "slider24:foo=0<0>hey";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min_str,
            max,
            max_str,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "24");
        assert!(matches!(identifier, Some(identifier) if identifier.as_str() == "foo"));
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(max_str, &None);
        assert_eq!(step_str, &None);
        assert_eq!(desc.as_str(), "hey");
        assert!(approx(*step, 0.0));
        assert!(approx(*max, 0.0));
        assert!(labels.is_none());
    }
    #[test]
    fn zero_param() {
        let source = "slider24:foo=0<>hey";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min,
            min_str,
            max,
            max_str,
            default_str,
            step,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "24");
        assert!(matches!(identifier, Some(identifier) if identifier.as_str() == "foo"));
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str, &None);
        assert_eq!(max_str, &None);
        assert_eq!(step_str, &None);
        assert_eq!(desc.as_str(), "hey");
        assert!(approx(*step, 0.0));
        assert!(approx(*max, 0.0));
        assert!(approx(*min, 0.0));
        assert!(labels.is_none());
    }
    #[test]
    fn no_closing_labels() {
        let source = "slider8:cfg_alert=0<0,3,1{off,yellow,red,yellow + red>-LUFS alerts";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min_str,
            max_str,
            default_str,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "8");
        assert!(matches!(identifier, Some(identifier) if identifier.as_str() == "cfg_alert"));
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "3");
        assert_eq!(
            step_str.as_ref().unwrap().as_str(),
            "1{off,yellow,red,yellow + red"
        );
        assert_eq!(desc.as_str(), "-LUFS alerts");
        let Some(labels) = labels else {
            panic!("Expected labels")
        };
        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0].as_str(), "off");
        assert_eq!(labels[1].as_str(), "yellow");
        assert_eq!(labels[2].as_str(), "red");
        assert_eq!(labels[3].as_str(), "yellow + red");
    }
    #[test]
    fn path() {
        let source = "slider1:/some_path:default_value:slider description";
        let (program, _) = File::lint_with_default_config(source);
        let Meta::SliderPath {
            id,
            default,
            desc,
            path,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert_eq!(id.as_str(), "1");
        assert_eq!(desc.as_str(), "slider description");
        assert_eq!(default.as_str(), "default_value");
        assert_eq!(path.as_str(), "/some_path");
    }

    #[test]
    fn incomplete() {
        let source = "slider24:0<1,10,1";
        let (program, issues) = File::lint_with_default_config(source);
        assert!(program.metas.is_empty());
        assert!(issues.has(&IssueKind::SliderParser));
    }

    #[test]
    fn incomplete2() {
        let source = "slider24:";
        let (program, _) = File::lint_with_default_config(source);
        assert!(program.metas.is_empty());
    }

    #[test]
    fn empty_identifier() {
        let source = "slider24:=0<1,10,1>Yo";
        let (program, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderInvalidIdentifier));
        assert_eq!(program.metas.len(), 1);
    }

    #[test]
    fn invalid_identifier() {
        let source = "slider24:hello world=0<1,10,1>Yo";
        let (program, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::SliderInvalidIdentifier));
        assert_eq!(program.metas.len(), 1);
    }
    #[test]
    fn without_step() {
        let source = "slider1:0<0,11,{1,1.5,2,2.5,3,4>Interval A";
        let (program, issues) = File::lint_with_default_config(source);
        let Meta::Slider {
            id,
            identifier,
            min_str,
            max_str,
            default_str,
            step_str,
            desc,
            labels,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert!(!issues.has(&IssueKind::SliderParser));
        assert_eq!(id.as_str(), "1");
        assert!(identifier.is_none());
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(min_str.as_ref().unwrap().as_str(), "0");
        assert_eq!(max_str.as_ref().unwrap().as_str(), "11");
        assert_eq!(step_str.as_ref().unwrap().as_str(), "{1,1.5,2,2.5,3,4");
        assert_eq!(desc.as_str(), "Interval A");
        let Some(labels) = labels else {
            panic!("Expected labels");
        };
        assert_eq!(labels.len(), 6);
        assert_eq!(labels[0].as_str(), "1");
        assert_eq!(labels[1].as_str(), "1.5");
        assert_eq!(labels[2].as_str(), "2");
        assert_eq!(labels[3].as_str(), "2.5");
        assert_eq!(labels[4].as_str(), "3");
        assert_eq!(labels[5].as_str(), "4");
    }

    #[test]
    fn no_braces() {
        let source = "slider1:0,Sample Rate";
        let (program, issues) = File::lint_with_default_config(source);
        assert!(!issues.has(&IssueKind::SliderParser));
        let Meta::Slider {
            id,
            identifier,
            default_str,
            desc,
            min,
            max,
            default,
            step,
            ..
        } = &program.metas[0]
        else {
            panic!("Expected a slider");
        };
        assert!(!issues.has(&IssueKind::SliderParser));
        assert_eq!(id.as_str(), "1");
        assert!(identifier.is_none());
        assert_eq!(default_str.as_str(), "0");
        assert_eq!(default, &0f64);
        assert_eq!(min, &0f64);
        assert_eq!(max, &0f64);
        assert_eq!(step, &0f64);
        assert_eq!(desc.as_str(), "Sample Rate");
    }

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }
}
