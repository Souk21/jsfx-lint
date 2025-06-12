use crate::access::var_kind::VarKind;
use crate::functions::{Arg, Depth, FunCall, Param};
use crate::rcsubstring::RcSubString;
use crate::section::Section;
use crate::{IssueKind, Program, functions::ParamKind, issue::IssueTracker};

/// Report value used as a namespace
pub fn lint(program: &Program, issues: &mut IssueTracker) {
    for section in program.sections.values() {
        for fun_call in &section.fun_calls {
            let Some(fun) = &fun_call.fun else {
                // Unknown function
                continue;
            };
            for (param_index, param_potentials) in fun_call.params.iter().enumerate() {
                let arg = fun.get_arg(param_index);
                let Some(arg) = arg else {
                    // Arg not found
                    continue;
                };
                // Ignore string args, because a number is accepted as well
                if !arg.is_ref || arg.is_str {
                    continue;
                }
                for param in param_potentials {
                    let param_is_value = matches!(
                        param.kind,
                        ParamKind::OtherValue | ParamKind::StringValue { .. }
                    );

                    let param_is_incorrect_arg =
                        param_is_incorrect_arg(param, fun_call, section, program);
                    // Ignore file_var() because it can accept both a namespace or a value
                    if (param_is_value || param_is_incorrect_arg)
                        && param_index < fun.args.len()
                        && fun.name.as_str() != "file_var"
                    {
                        issues.add(
                            IssueKind::ArgMustBeNamespace,
                            &param.location,
                            format!(
                                "{}() arg {} must be a namespace",
                                fun.name, fun.args[param_index].name
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn param_is_incorrect_arg(
    param: &Param,
    fun_call: &FunCall,
    section: &Section,
    program: &Program,
) -> bool {
    let ParamKind::Identifier { name } = &param.kind else {
        return false;
    };
    let Depth::Nested { parent_fun: parent } = fun_call.depth else {
        return false;
    };
    let parent = section
        .uuid_to_fun_defs
        .get(&parent)
        .or_else(|| {
            program
                .sections
                .get("init")
                .map(|s| s.uuid_to_fun_defs.get(&parent))?
        })
        .expect("Parent was not found");
    let name_rc = RcSubString::from_str(name.as_str());
    match parent.classify_variable(&name_rc, false) {
        VarKind::RefArg { .. }
        | VarKind::Instance { .. }
        | VarKind::TempString
        | VarKind::Local
        | VarKind::This { .. } => false,
        VarKind::Arg { arg_index } => {
            let arg = parent.args.get(arg_index);
            !matches!(arg, Some(Arg { is_ref: true, .. }))
        }
        VarKind::Global { accessible } => !accessible,
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use indoc::indoc;
    #[test]
    fn value_as_namespace() {
        let source = indoc! {"
            @init
            function foo(bar*) ( 0; );
            foo(0);
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ArgMustBeNamespace));
    }
    #[test]
    fn function_as_namespace() {
        let source = indoc! {"
            @init
            function foo(bar*) ( 0; );
            function baz() ( 0; );
            foo(baz());
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ArgMustBeNamespace));
    }

    #[test]
    fn arg_as_namespace() {
        let source = indoc! {"
            @init
            function foo(baz*) ( baz = 2; );
            function bar(bal) ( foo(bal); );
        "};
        let (_, issues) = File::lint_with_default_config(source);
        assert!(issues.has(&IssueKind::ArgMustBeNamespace));
    }
}
