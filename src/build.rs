use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::{collections::BTreeMap, error::Error};
use std::{env, path::PathBuf};

use cfgrammar::yacc::YaccKind;
use lrlex::CTLexerBuilder;
use quote::{format_ident, quote};
use toml::Table;
use uuid::Uuid;

pub const MAX_TRACK_CHANNEL_COUNT: usize = 128;
pub const MAX_SLIDER_COUNT: usize = 256;
pub const SEVERITIES: [&str; 4] = ["silent", "style", "warning", "error"];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Not ideal, copy eel_pp to build target directory
    copy_eel_pp_to_output().expect("Could not copy eel_pp to output directory");
    CTLexerBuilder::new()
        .lrpar_config(|ctp| {
            ctp.yacckind(YaccKind::Grmtools)
                .grammar_in_src_dir("../grammar/eel2.y")
                .unwrap()
        })
        .lexer_in_src_dir("../grammar/eel2.l")?
        .build()?;

    generate_functions()?;
    generate_variables()?;
    generate_globals()?;
    generate_config()?;
    Ok(())
}

fn get_writer(file_name: &str) -> Result<BufWriter<File>, Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join(file_name);
    Ok(BufWriter::new(File::create(dest_path)?))
}

fn ascii_capitalize(s: &str) -> String {
    let mut str = s.to_string();
    if let Some(r) = str.get_mut(0..1) {
        r.make_ascii_uppercase();
    }
    str
}

fn snake_to_pascal_case(input: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in input.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

fn generate_config() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = get_writer("config.rs")?;
    let default_config_path = env::current_dir()?.join("data/config.default.toml");
    let default_config_str = fs::read_to_string(default_config_path)?;
    let table = default_config_str.parse::<Table>()?;
    let mut issue_kinds = vec![];
    // Use a BTreeMap to ensure a consistent ordering of the generated enum variants
    let mut severity_map = BTreeMap::new();
    let mut string_to_severity = Vec::new();
    let mut config_lines = Vec::new();
    let mut string_to_issue_kind = Vec::new();
    let mut issue_kind_to_string = Vec::new();
    let mut severities_enum = Vec::new();

    for severity in SEVERITIES {
        let severity_ident = format_ident!("{}", ascii_capitalize(severity));
        severities_enum.push(severity_ident.clone());
        string_to_severity.push(quote! {
            #severity => Some(Severity::#severity_ident)
        });
    }

    for (issue_kind, severity) in table.iter() {
        let Some(severity) = severity.as_str() else {
            return Err("Value is not a string".into());
        };
        if !SEVERITIES.contains(&severity) {
            return Err("Value is not a valid severity".into());
        }
        let severity_ident = format_ident!("{}", ascii_capitalize(severity));
        let issue_kind_ident = format_ident!("{}", snake_to_pascal_case(issue_kind));
        string_to_issue_kind.push(quote! {
            #issue_kind => Some(IssueKind::#issue_kind_ident)
        });
        issue_kind_to_string.push(quote! {
            &IssueKind::#issue_kind_ident => #issue_kind
        });
        issue_kinds.push(issue_kind_ident.clone());
        config_lines.push(quote! {
            config.insert(IssueKind::#issue_kind_ident, Severity::#severity_ident);
        });
        severity_map.insert(issue_kind_ident, severity_ident);
    }

    let config = quote! {
        #[derive(Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Clone)]
        pub enum Severity {
            #(#severities_enum,)*
        }
        #[derive(Eq, Hash, PartialEq, Debug)]
        pub enum IssueKind {
            #(#issue_kinds,)*
        }
        pub fn get_default_config() -> HashMap<IssueKind, Severity> {
            let mut config = HashMap::new();
            #(#config_lines)*
            config
        }
        pub fn string_to_severity(s: &str) -> Option<Severity> {
            match s {
                #(#string_to_severity),*,
                _ => None,
            }
        }
        pub fn string_to_issue_kind(s: &str) -> Option<IssueKind> {
            match s {
                #(#string_to_issue_kind),*,
                _ => None,
            }
        }
        pub fn issue_kind_to_string(kind: &IssueKind) -> &'static str {
            match kind {
                #(#issue_kind_to_string),*,
            }
        }
    };

    writeln!(writer, "{}", config)?;
    Ok(())
}

fn generate_globals() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = get_writer("globals.rs")?;
    let globals = quote! {
        pub const MAX_TRACK_CHANNEL_COUNT: usize = #MAX_TRACK_CHANNEL_COUNT;
        pub const MAX_SLIDER_COUNT: usize = #MAX_SLIDER_COUNT;
        pub const SECTION_KINDS: [&str; 6] = ["init", "sample", "block", "slider", "serialize", "gfx"];
    };
    write!(writer, "{}", globals)?;
    Ok(())
}

fn generate_variables() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = get_writer("variables.rs")?;
    let path = env::current_dir()?.join("data/variables");
    let data = fs::read_to_string(path)?;
    let mut variables = Vec::new();
    for line in data.lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with('!') {
            continue;
        }
        let mut parsing_rw = false;
        let mut last_index = 0;
        let mut readable = false;
        let mut writable = false;
        let mut name = None;
        let mut contexts = None;
        for (index, char) in line.char_indices() {
            if name.is_none() && char == ' ' {
                name = Some(&line[0..index]);
                last_index = index;
            } else if char == '[' {
                parsing_rw = true;
            } else if parsing_rw && char == ']' {
                parsing_rw = false;
                last_index = index + 1;
            } else if parsing_rw && char == 'r' {
                readable = true;
            } else if parsing_rw && char == 'w' {
                writable = true;
            } else if !parsing_rw && name.is_some() {
                let parsed_contexts = if line[last_index..line.len()].trim().starts_with('-') {
                    vec!["sample", "block", "slider", "serialize", "gfx"]
                } else {
                    line[last_index..line.len()]
                        .split(',')
                        .map(|context_str| context_str.trim().trim_matches('"'))
                        .collect()
                };
                contexts = Some(quote! {
                    vec![#(#parsed_contexts),*]
                });
                break;
            }
        }
        let contexts = if let Some(contexts) = contexts {
            quote! {
                Some(Context::from_vec(#contexts))
            }
        } else {
            quote! {
                None
            }
        };
        variables.push(quote! {
            (String::from(#name), Rc::new(BuiltinVar {
                name: String::from(#name),
                context: #contexts,
                writable: #writable,
                readable: #readable,
            })),
        });
    }
    // Add `splN` variables
    for i in 0..MAX_TRACK_CHANNEL_COUNT {
        let name = format!("spl{i}");
        variables.push(quote! {
            (String::from(#name), Rc::new(BuiltinVar {
                name: String::from(#name),
                context: Some(Context::from_vec(vec!["sample"])),
                writable: true,
                readable: true,
            })),
        });
    }
    // Add `sliderN` variables
    for i in 1..=MAX_SLIDER_COUNT {
        let name = format!("slider{i}");
        variables.push(quote! {
            (String::from(#name), Rc::new(BuiltinVar {
                name: String::from(#name),
                context: None,
                writable: true,
                readable: true,
            })),
        });
    }
    // Add `regN` variables (note there's a leading zero in reg01 -> reg09)
    for i in 0..100 {
        let name = format!("reg{i:0>2}");
        variables.push(quote! {
            (String::from(#name), Rc::new(BuiltinVar {
                name: String::from(#name),
                context: None,
                writable: true,
                readable: true,
            })),
        });
    }
    let result = quote! {
        use crate::functions::Arg;
        use crate::variables::BuiltinVar;
        fn get_builtin_vars() -> HashMap<String, Rc<BuiltinVar>> {
            HashMap::from([
                #(#variables)*
            ])
        }
    };
    write!(writer, "{}", result)?;
    Ok(())
}

fn generate_functions() -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = get_writer("functions.rs")?;
    let path = env::current_dir()?.join("data/functions");
    let data = fs::read_to_string(path)?;
    let mut functions = Vec::new();
    for line in data.lines() {
        if line.is_empty() {
            continue;
        }
        let mut name = None;
        let mut parsed_until = 0;
        let mut args = Vec::new();
        let mut optional = false;
        let mut contexts: Option<Vec<&str>> = None;
        let mut has_side_effects = false;
        for (index, char) in line.char_indices() {
            if index == 0 && char == '*' {
                has_side_effects = true;
                parsed_until = 1;
                continue;
            }
            if name.is_none() {
                if char == '(' {
                    let start = if has_side_effects { 1 } else { 0 };
                    name = Some(&line[start..index]);
                    parsed_until = index + 1;
                }
                continue;
            }
            if char == ',' || char == ']' || char == ')' {
                let mut arg = line[parsed_until..index].trim();
                if !arg.is_empty() {
                    // Keep the potential `#` but not the potential `*`
                    let is_str = arg.starts_with('#');
                    let is_ref = arg.ends_with('*');
                    if is_ref {
                        arg = &arg[0..arg.len() - 1];
                    }
                    args.push(quote! {
                        Arg {
                            location: None,
                            name: RcSubString::from_str(#arg),
                            is_str: #is_str,
                            optional: #optional,
                            is_ref: #is_ref,
                        }
                    });
                }
                parsed_until = index + 1;
                if char != ')' {
                    continue;
                }
                let context_str = &line[parsed_until..line.len()].trim();
                if !context_str.is_empty() {
                    contexts = Some(
                        context_str
                            .split(',')
                            .map(|context| context.trim().trim_matches('"'))
                            .collect(),
                    );
                }
                let contexts = if let Some(contexts) = &contexts {
                    quote! {
                        MaybeContext::Some(Context::from_vec(vec![#(#contexts),*]))
                    }
                } else {
                    quote! {
                        MaybeContext::None
                    }
                };
                let uuid = Uuid::new_v4().to_string();
                functions.push(quote! {
                    Rc::new(Fun {
                        uuid: uuid!(#uuid),
                        is_builtin: true,
                        location: None,
                        modifiers: HashMap::new(),
                        name: RcSubString::from_str(#name),
                        args: vec![#(#args),*],
                        context: #contexts,
                        scope: FunScope::new(),
                        has_side_effects: #has_side_effects,
                    })
                });
                break;
            }
            if !optional && char == '[' {
                optional = true;
                parsed_until = index + 1;
                continue;
            }
        }
    }
    let result = quote! {
        use uuid::uuid;
        use std::rc::Rc;
        use crate::context::{Context, MaybeContext};
        use crate::functions::Fun;
        use crate::scopes::FunScope;
        pub fn get_builtin_funs() -> Vec<Rc<Fun>> {
            vec![
                #(#functions),*
            ]
        }
    };

    write!(writer, "{}", result)?;
    Ok(())
}

fn get_eel_pp_path() -> &'static str {
    if cfg!(target_os = "windows") {
        "bin/eel_pp_win.exe"
    } else if cfg!(target_os = "linux") {
        "bin/eel_pp_linux"
    } else if cfg!(target_os = "macos") {
        "bin/eel_pp_macos"
    } else {
        panic!("Unsupported target os")
    }
}

fn get_eel_pp_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "eel_pp.exe"
    } else if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        "eel_pp"
    } else {
        panic!("Unsupported target os")
    }
}

pub fn copy_eel_pp_to_output() -> Result<(), Box<dyn Error>> {
    // Original is from the crate "copy_to_output" https://crates.io/crates/copy_to_output/
    // Adapted to use fs::copy and rename the file.
    let build_type = &env::var("PROFILE")?;
    let mut out_path = PathBuf::new();
    out_path.push("target");

    // This is a hack, ideally we would plug into https://docs.rs/cargo/latest/cargo/core/compiler/enum.CompileKind.html
    // However, since the path follows predictable rules https://doc.rust-lang.org/cargo/guide/build-cache.html
    // we can just check our parent path for the pattern target/{triple}/{profile}.
    // If it is present, we know CompileKind::Target was used, otherwise CompileKind::Host was used.
    // Best effort since the existing tests aren't intended to be run in a real build this won't exist.
    // Unclear if that also means people in the wild are using the crate similarly, so avoiding any risk of break.
    if let Ok(triple) = std::env::var("TARGET")
        && let Some(out_dir) = env::var_os("OUT_DIR")
        && let Some(out_dir) = out_dir.to_str()
        && out_dir.contains(&format!("target{}{}", std::path::MAIN_SEPARATOR, triple))
    {
        out_path.push(triple);
    }
    out_path.push(build_type);
    out_path.push(get_eel_pp_name());
    fs::copy(get_eel_pp_path(), out_path)?;
    Ok(())
}
