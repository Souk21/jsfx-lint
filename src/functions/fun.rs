use std::rc::Rc;

use crate::context::MaybeContext;
use crate::functions::{
    Arg, ArgIndex, Fun, ModifierKind, global_matches_arg, has_assignment_to_mem_access,
    identifier_refers_to_fun_ref_arg,
};
use crate::located_ast::LocatedAst;
use crate::rcsubstring::RcSubString;
use crate::scopes::FunScope;
use crate::{access, ast::Ast};
use crate::{access::var_kind::VarKind, scopes::GlobalScope, section::Section};

use super::FunCall;

impl Fun {
    /// Bootstraps a `Fun` that still needs to have symbols/scope collected etc
    pub fn start_setup(ast_loc: &LocatedAst, fun_calls_in_this_fn: &[FunCall]) -> Self {
        let Ast::Fun {
            identifier,
            args: args_opt,
            modifiers: modifiers_opt,
            uuid,
            body,
            ..
        } = &ast_loc.ast
        else {
            panic!("Expected AST::Fn");
        };
        let fun_name = identifier
            .ast
            .identifier()
            .expect("Non identifier in identifier position");
        let args_vec = args_opt
            .as_ref()
            .map_or_else(Vec::new, super::args::collect_args);
        let modifiers = super::modifiers::collect_fn_mods(modifiers_opt);
        // This is incomplete, because write accesses are not yet collected (they are collected in `finish_setup`)
        let has_side_effects = has_assignment_to_mem_access(body)
            || fun_calls_in_this_fn
                .iter()
                .map(|fun_call| fun_call.fun.as_ref())
                .any(|fun| fun.is_some_and(|fun| fun.has_side_effects));
        Self {
            name: fun_name.clone(),
            args: args_vec,
            // Context will be set after the scope is collected, in `finish_setup`
            context: MaybeContext::Unknown,
            modifiers,
            location: Some(identifier.location.clone()),
            is_builtin: false,
            uuid: *uuid,
            scope: FunScope::new(),
            has_side_effects,
        }
    }

    pub fn finish_setup(
        &mut self,
        body: &LocatedAst,
        section: &Section,
        fun_calls_in_this_fn: &Vec<Rc<FunCall>>,
        scope: &GlobalScope,
    ) {
        // Get accesses and returns from function body
        let mut fun_accesses = Vec::new();
        let fun_returns = access::get_accesses(body, &mut fun_accesses, section);
        for access in &fun_accesses {
            self.scope.add_access(access.to_within_function(&*self));
        }
        // Function returns need to be marked as read in the function here,
        // because an identifier as the last expression of a compound statement is not automatically marked as a read.
        let mut return_accesses = Vec::new();
        access::fun_returns_to_read_accesses(&fun_returns, &mut return_accesses);
        for fun_return in return_accesses {
            self.scope.add_access(fun_return.to_within_function(&*self));
        }
        // Transform potential Return::Named to Return::Value, as `function.returns` needs to be Return::Value
        let fun_returns: Vec<_> = fun_returns
            .unwrap_or_default()
            .iter()
            .map(|ret| ret.named_to_value(&fun_accesses))
            .collect();
        self.scope.returns = fun_returns;
        self.has_side_effects =
            self.has_side_effects || fun_accesses.iter().any(|access| access.info.is_write());
        self.context = super::context::collect(fun_calls_in_this_fn, &scope.builtin_vars, &*self);
    }

    pub fn get_arg(&self, index: usize) -> Option<&Arg> {
        if index < self.args.len() {
            Some(&self.args[index])
        } else if self.is_builtin {
            self.args.last().and_then(|last| {
                if last.name.as_str() == "..." {
                    Some(last)
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    fn find_fun_arg(&self, identifier: &RcSubString) -> Option<(ArgIndex, &Arg)> {
        self.args
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.name.to_lower() == identifier.to_lower())
    }

    pub(crate) fn has_modifier_with_arg_name(
        &self,
        kind: &ModifierKind,
        name: &RcSubString,
    ) -> bool {
        self.modifiers.get(kind).is_some_and(|mods| {
            mods.iter().any(|modifier| {
                modifier
                    .args
                    .iter()
                    .any(|arg| arg.name.to_lower() == name.to_lower())
            })
        })
    }

    fn has_local_var(&self, name: &str) -> bool {
        let Some(local_mod) = self.modifiers.get(&ModifierKind::Local) else {
            // No local modifier
            return false;
        };
        let name_lower = name.to_ascii_lowercase();
        for modifier in local_mod {
            for arg in &modifier.args {
                // Note: local variables are considered "ref" by default
                if arg.name.to_lower() == name_lower {
                    return true;
                }
            }
        }
        false
    }

    fn find_instance_var(&self, var_name: &RcSubString) -> Option<Option<RcSubString>> {
        let Some(instance_mods) = self.modifiers.get(&ModifierKind::Instance) else {
            // No instance modifier
            return None;
        };
        let var_name_lower = var_name.to_ascii_lowercase();
        for modifier in instance_mods {
            for arg in &modifier.args {
                // Note: instance variables can end with a dot
                // Note: instance variables are considered "ref" by default
                let arg_name_lower = arg.name.to_lower();
                if arg_name_lower == var_name_lower {
                    // No suffix
                    return Some(None);
                }
                if var_name_lower.starts_with(arg_name_lower)
                    && var_name_lower.chars().nth(arg_name_lower.len()) == Some('.')
                {
                    let suffix = var_name.substr(arg_name_lower.len() + ".".len()..);
                    return Some(Some(suffix));
                }
            }
        }
        None
    }

    pub fn find_fun_ref_arg(&self, identifier: &RcSubString) -> Option<(RcSubString, ArgIndex)> {
        self.args
            .iter()
            .enumerate()
            .find(|(_, arg)| identifier_refers_to_fun_ref_arg(arg, identifier))
            .map(|(arg_index, arg)| {
                let suffix = identifier
                    .extract_suffix(&arg.name)
                    .expect("identifier should start with arg name");
                (suffix, arg_index)
            })
    }

    pub fn classify_variable(
        &self,
        variable: &RcSubString,
        bypass_global_modifier: bool,
    ) -> VarKind {
        if variable.as_str() == "#" {
            VarKind::TempString
        } else if let (true, suffix) = get_suffix_if_this(variable) {
            VarKind::This { suffix }
        } else if let Some((suffix, arg_index)) = self.find_fun_ref_arg(variable) {
            VarKind::RefArg { suffix, arg_index }
        } else if let Some((arg_index, _)) = self.find_fun_arg(variable) {
            VarKind::Arg { arg_index }
        } else if self.has_local_var(variable) {
            VarKind::Local
        } else if let Some(suffix) = self.find_instance_var(variable) {
            VarKind::Instance { suffix }
        } else {
            VarKind::Global {
                accessible: bypass_global_modifier || self.global_var_is_accessible(variable),
            }
        }
    }

    pub fn global_var_is_accessible(&self, identifier: &RcSubString) -> bool {
        let Some(global_mods) = &self.modifiers.get(&ModifierKind::Global) else {
            // No global modifier, so all global vars are accessible
            return true;
        };
        let flat_global_mods = global_mods
            .iter()
            .flat_map(|modifier| &modifier.args)
            .collect::<Vec<_>>();
        if flat_global_mods.is_empty() {
            // All global modifiers have no arg, so no global vars are accessible
            return false;
        }
        // If var is in at least one global modifier, it's accessible
        flat_global_mods
            .iter()
            .any(|arg| global_matches_arg(arg, identifier))
    }
}

fn get_suffix_if_this(identifier: &RcSubString) -> (bool, Option<RcSubString>) {
    if identifier.to_lower() == "this" {
        (true, None)
    } else if let Some(prefix) = identifier.strip_prefix_case("this.") {
        (true, Some(prefix))
    } else {
        (false, None)
    }
}
