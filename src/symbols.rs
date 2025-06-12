use std::{collections::HashMap, rc::Rc};

use uuid::Uuid;

use crate::access;
use crate::first_pass::FirstPass;
use crate::functions::{Fun, FunCall};
use crate::iterators::ast_walk_signal::AstWalkSignal;
use crate::scopes::GlobalScope;
use crate::section::Section;
use crate::{get_builtin_funs, get_builtin_vars};

pub fn collect(first_pass: &mut FirstPass) -> GlobalScope {
    let builtins = get_builtin_funs();
    let mut scope = GlobalScope::new(get_builtin_vars());

    // First, collect symbols in @init
    if let Some(init_sec) = first_pass.sections.get_mut("init") {
        collect_section(init_sec, &None, &builtins, &mut scope);
    }

    // Remove the @init section from the `HashMap`,
    // to be able to mutably borrow the other sections while having an `init_sec` immutable borrow
    let init_sec = first_pass.sections.remove("init");

    // Then collect symbols in all other sections
    for section in first_pass.sections.values_mut() {
        collect_section(section, &init_sec, &builtins, &mut scope);
    }

    // Put back the @init section in the `HashMap`
    if let Some(init_sec) = init_sec {
        first_pass.sections.insert("init", init_sec);
    }

    scope
}

/// Add a function call to a section
/// Note: Because of where this function is used, it's not practical to mutably borrow the full `section`.
/// Hence, `section.fun_calls` and `section.uuid_to_fun_calls` are borrowed separately.
fn add_fun_call_to_section(
    fun_call: FunCall,
    fun_calls: &mut Vec<Rc<FunCall>>,
    uuid_to_fun_calls: &mut HashMap<Uuid, Rc<FunCall>>,
) -> Rc<FunCall> {
    let fun_call_rc = Rc::new(fun_call);
    fun_calls.push(fun_call_rc.clone());
    uuid_to_fun_calls.insert(fun_call_rc.uuid, fun_call_rc.clone());
    fun_call_rc
}

fn collect_section(
    section: &mut Section,
    init_section: &Option<Section>,
    builtins: &[Rc<Fun>],
    scope: &mut GlobalScope,
) {
    for chunk in &section.chunks {
        let mut fun_calls_in_this_fn: Vec<FunCall> = Vec::new();
        let mut is_in_fn = false;
        let Some(root) = chunk.ast.as_ref() else {
            // Chunk was not fully parsed, skip it
            continue;
        };
        for ast_sig in AstWalkSignal::new(root) {
            if ast_sig.is_entering_function() {
                is_in_fn = true;
            } else if let Some((fun_ast, body_ast)) = ast_sig.is_exiting_function() {
                is_in_fn = false;
                let mut fun = Fun::start_setup(fun_ast, &fun_calls_in_this_fn);
                let mut fun_calls = Vec::new();
                // Finish setup for function calls in this function
                for mut fun_call in fun_calls_in_this_fn.drain(..) {
                    fun_call.finish_setup(Some(&fun));
                    let fun_call_rc = add_fun_call_to_section(
                        fun_call,
                        &mut section.fun_calls,
                        &mut section.uuid_to_fun_calls,
                    );
                    fun_calls.push(fun_call_rc);
                }
                fun.finish_setup(body_ast, section, &fun_calls, scope);
                let fun = Rc::new(fun);
                section.fun_defs.push(fun.clone());
                section.uuid_to_fun_defs.insert(fun.uuid, fun.clone());
            } else if let Some((id, params, uuid)) = ast_sig.is_entering_fun_call() {
                let mut fun_call = FunCall::start_setup(
                    params,
                    id,
                    section,
                    init_section,
                    builtins,
                    is_in_fn,
                    uuid,
                );
                if is_in_fn {
                    // Wait for the outer function to be fully parsed to finish setup
                    fun_calls_in_this_fn.push(fun_call);
                } else {
                    // Function call is top level, no need to wait for surrounding scope
                    fun_call.finish_setup(None);
                    add_fun_call_to_section(
                        fun_call,
                        &mut section.fun_calls,
                        &mut section.uuid_to_fun_calls,
                    );
                }
            }
        }
        let mut top_level_accesses = Vec::new();
        access::get_accesses(root, &mut top_level_accesses, section);
        for access in &top_level_accesses {
            scope.add_access(access, section.kind);
        }
    }
}
