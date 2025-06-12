use crate::access::var_kind::VarKind;
use crate::context::{Context, ContextDemander, MaybeContext};
use crate::functions::{Fun, FunCall};
use crate::variables::BuiltinVar;
use std::{collections::HashMap, rc::Rc};

pub fn collect(
    fun_calls_in_this_fn: &Vec<Rc<FunCall>>,
    builtin_vars: &HashMap<String, Rc<BuiltinVar>>,
    fun: &Fun,
) -> MaybeContext {
    // The context of a function is the intersection (&) of the contexts
    // of every function it calls and every builtin variable it accesses
    let mut context = Context::new();
    let mut contexts: Vec<ContextDemander> = Vec::new();

    // Check context for function calls
    for called_fun in fun_calls_in_this_fn {
        let Some(called_fun) = &called_fun.fun else {
            // Called function is unknown
            continue;
        };

        match &called_fun.context {
            // Called function doesn't have context, so it's compatible with any context
            MaybeContext::None => continue,
            // Called function has unknown/incompatible context, so the context of this function is unknown
            MaybeContext::Unknown | MaybeContext::HasIncompatibleDemanders(_) => {
                return MaybeContext::Unknown;
            }
            MaybeContext::Some(called_fun_context) => {
                contexts.push(ContextDemander::FunctionCall {
                    context: called_fun_context.clone(),
                    fun_name: called_fun.name.clone(),
                });
                context = context.intersect(called_fun_context);
                if context.is_empty() {
                    return MaybeContext::HasIncompatibleDemanders(contexts);
                }
            }
        }
    }

    // Check context for builtins variables
    for access in &fun.scope.accesses {
        let VarKind::Global { .. } = &access.var_kind else {
            // A builtin variable has to be a global variable
            continue;
        };
        // Not `GlobalScope::is_builtin` here because Meta have no need to be considered here
        let builtin = builtin_vars.get(access.info.accessed_as.to_lower());
        let Some(builtin) = builtin else {
            // Variable is not a builtin
            continue;
        };
        let Some(builtin_context) = &builtin.context else {
            // Variable has no context
            continue;
        };
        contexts.push(ContextDemander::Variable(builtin.clone()));
        context = context.intersect(builtin_context);
        if context.is_empty() {
            return MaybeContext::HasIncompatibleDemanders(contexts);
        }
    }

    if context.is_compatible_with_all() {
        MaybeContext::None
    } else {
        MaybeContext::Some(context)
    }
}
