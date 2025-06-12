use std::{collections::HashMap, rc::Rc};

use uuid::Uuid;

use crate::iterators::ast_walk::AstWalk;
use crate::{
    ast::Ast, context::MaybeContext, located_ast::LocatedAst, location::Location,
    rcsubstring::RcSubString, scopes::FunScope, section::Section,
};

mod args;
pub mod context;
mod fun;
mod fun_call;
mod modifiers;
pub mod params;
mod tests;

#[derive(Debug)]
pub enum ParamKind {
    Identifier {
        name: RcSubString,
    },
    StringValue {
        value: String,
    },
    /// Value can be a number, a function call, a compound statement etc.
    OtherValue,
}

#[derive(Debug)]
pub struct Param {
    pub kind: ParamKind,
    pub location: Location,
}

#[derive(Debug)]
pub enum Depth {
    TopLevel,
    Nested {
        /// Parent `FunDef`
        parent_fun: Uuid,
    },
    /// Depth is undetermined until the outermost function is fully collected
    Undetermined,
}

#[derive(Debug)]
pub struct FunCall {
    /// Each param `Ast` can have multiple return values, if it contains a conditional.
    /// e.g. `foo(a ? b : c)` first param returns `b` and `c`
    pub params: Vec<Vec<Param>>,
    /// If `fun` is `None`, that means the called function is unknown
    pub fun: Option<Rc<Fun>>,
    /// Prefix at call site, without the dot
    /// e.g. `foo` in `foo.bar()`
    pub prefix: Option<RcSubString>,
    pub uuid: Uuid,
    pub location: Location,
    /// The function call as it appears in the source code. (e.g. `foo.bar()` is accessed as `foo.bar`)
    pub accessed_as: RcSubString,
    pub depth: Depth,
    /// In this situation:
    /// ```
    /// function bar() (
    ///     this.inner = 1;
    /// );
    /// function foo() global(bar.hello) (
    ///     bar.hello();
    /// );
    /// ```
    /// Where `bar*` should be inaccessible,
    /// but is accessible because a global arg has the same name as the prefix + the function name
    pub name_matches_global_arg: bool,
    /// In this situation:
    /// ```
    /// function bar() (
    ///     this.inner = 1;
    /// );
    /// function foo() instance(bar) (
    ///     bar();
    /// );
    /// object.foo();
    /// ```
    /// Where it looks like `object.bar.inner` should be set,
    /// but is not because the function call `bar()` has no prefix and its name corresponds to an instance arg.
    /// So here, `object.inner` is set.
    /// Note: this can't be shadowed by other modifiers.
    /// Note that this does not work with function call prefix:
    /// ```
    /// function bar() (
    ///     this.inner = 1;
    /// );
    /// function foo() instance(boo.bar) (
    ///     boo.bar();
    /// );
    /// object.foo(); // object.boo.inner gets set
    /// ```
    pub name_matches_instance_arg: bool,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Arg {
    /// Name of the argument, as it appears in the function definition, without the potential `*` but with the potential `#`
    pub name: RcSubString,
    pub is_ref: bool,
    pub optional: bool,
    pub is_str: bool,
    pub location: Option<Location>,
}

#[derive(Debug)]
pub struct Modifier {
    pub args: Vec<Arg>,
    pub location: Location,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum ModifierKind {
    Global,
    Local,
    Instance,
}

#[derive(Debug)]
pub struct Fun {
    pub name: RcSubString,
    pub args: Vec<Arg>,
    pub modifiers: HashMap<ModifierKind, Vec<Modifier>>,
    pub context: MaybeContext,
    pub location: Option<Location>,
    pub is_builtin: bool,
    pub uuid: Uuid,
    pub scope: FunScope,
    /// A function has side effects if its body includes any write accesses,
    /// or calls to functions with side effects
    pub has_side_effects: bool,
}

type ArgIndex = usize;

pub struct MatchingFun {
    pub fun: Option<Rc<Fun>>,
    pub prefix: Option<RcSubString>,
}

fn extract_prefix_if_exists(
    name: &RcSubString,
    prefix: &mut Option<RcSubString>,
    matching_function: &Option<Rc<Fun>>,
) {
    if let Some(found) = &matching_function {
        let this_prefix = name.substr(0..name.len() - found.name.len() - 1);
        *prefix = Some(this_prefix);
    }
}

fn global_matches_arg(arg: &Arg, identifier: &RcSubString) -> bool {
    // Note: in `global()` modifier only ref args "children" are accessible
    // e.g. `global(foo*)` allows access to `foo.bar` but not `foo`
    //
    // Note: identifier can end with a dot and refer to the ref arg
    // e.g.
    // ```
    // function foo() global(bar*) ( bar. = 1; );
    // foo(); // bar. = 1
    // ```
    let arg_name_lower = arg.name.to_lower();
    let identifier_lower = identifier.to_lower();
    let matches_non_ref = !arg.is_ref && arg_name_lower == identifier_lower;
    let matches_ref = arg.is_ref && identifier_lower.starts_with(&format!("{arg_name_lower}."));
    matches_non_ref || matches_ref
}

fn identifier_refers_to_fun_ref_arg(arg: &Arg, identifier: &RcSubString) -> bool {
    // Note: Contrary to `instance()` args and `global()` ref args,
    // access to function ref args can *NOT* end with a dot
    // e.g. `function foo(bar*) global() ( bar. = 1; );`
    // JSFX complains "global bar. inaccessible"
    if !arg.is_ref {
        return false;
    }
    let arg_name_lower = arg.name.to_lower();
    let identifier_lower = identifier.to_lower();
    let exact_match = arg_name_lower == identifier_lower;
    let ref_match = identifier_lower.starts_with(&format!("{arg_name_lower}."))
        && identifier_lower.len() > arg_name_lower.len() + ".".len() + 1;
    exact_match || ref_match
}

pub fn find_matching_function(
    section: &Section,
    name: &RcSubString,
    param_count: usize,
    init_section: &Option<Section>,
    builtins: &[Rc<Fun>],
) -> MatchingFun {
    let mut prefix = None;
    // First, try to find a fn with exact name and param count in this section
    let fun = section
        .find_exact_function(name, param_count)
        // Or try to find a fn with exact name and param count in "init" section
        .or_else(|| {
            init_section
                .as_ref()
                .and_then(|init_sec| init_sec.find_exact_function(name, param_count))
        })
        // Or try to find a fn with exact name and param count in builtins
        .or_else(|| find_exact_builtin(name, param_count, builtins))
        // Or try to find the longest "instance" fn in both "init" and this section
        .or_else(|| {
            let mut matching_function = None;
            let init_found = init_section
                .as_ref()
                .and_then(|init_sec| init_sec.find_exact_obj_function(name, param_count));
            let sec_found = section.find_exact_obj_function(name, param_count);
            match (init_found.as_ref(), sec_found.as_ref()) {
                (Some(init), Some(sec)) => {
                    if init.name.len() > sec.name.len() {
                        matching_function = init_found;
                    } else {
                        matching_function = sec_found;
                    }
                }
                (Some(_), None) => matching_function = init_found,
                (None, Some(_)) => matching_function = sec_found,
                _ => {}
            }
            extract_prefix_if_exists(name, &mut prefix, &matching_function);
            matching_function
        })
        // Or find a function with exact name in this section that doesn't have the correct number of params
        //IDEA here JSFX proposes all the different matching function (eg. foo() expects 2, 3 or 6 params)
        .or_else(|| section.find_inexact_function(name))
        // Same for @init
        .or_else(|| {
            init_section
                .as_ref()
                .and_then(|init_sec| init_sec.find_inexact_function(name))
        })
        // Or find the longest "instance" fn in this section that doesn't have the correct number of params
        .or_else(|| {
            let matching_function = section.find_inexact_obj_function(name);
            extract_prefix_if_exists(name, &mut prefix, &matching_function);
            matching_function
        })
        // Same for @init
        .or_else(|| {
            let mut matching_function = None;
            if let Some(init_sec) = &init_section {
                matching_function = init_sec.find_inexact_obj_function(name);
                extract_prefix_if_exists(name, &mut prefix, &matching_function);
            }
            matching_function
        })
        // Or find an inexact match in builtins
        .or_else(|| find_inexact_builtin(name, builtins));
    MatchingFun { fun, prefix }
}

fn has_assignment_to_mem_access(body: &LocatedAst) -> bool {
    AstWalk::new(body).any(|ast_loc| {
        let Ast::Assignment { lhs, .. } = &ast_loc.ast else {
            return false;
        };
        lhs.get_return_values()
            .iter()
            .any(|ret| matches!(ret.ast, Ast::MemoryAccess { .. }))
    })
}

pub fn get_args_min_max(args: &Vec<Arg>) -> (usize, usize) {
    let mut min = 0;
    let mut max = 0;
    for arg in args {
        if arg.name.as_str() == "..." {
            max = 32;
        } else if arg.optional {
            max += 1;
        } else {
            min += 1;
            max += 1;
        }
    }
    max = max.clamp(0, 32);
    min = min.clamp(0, 32);
    (min, max)
}

pub fn match_arg_count(param_count: usize, args: &Vec<Arg>) -> bool {
    // Note: When resolving fun call, due to implicit param passing, this situation arises:
    // ```
    // function foo() {};
    // function foo(a) {};
    // foo(); // foo(a) is called with a = 0
    // ```
    // And due to implicit param discarding, this situation arises:
    // ```
    // function foo(a) {};
    // function foo() {};
    // foo(1); // foo() is called and '1' is discarded
    // ```
    let (min, max) = get_args_min_max(args);
    let implicit_zero = param_count == 0 && min == 1 && max == 1;
    let implicit_discard = param_count == 1 && min == 0 && max == 0;
    let normal_match = param_count >= min && param_count <= max;
    implicit_zero || implicit_discard || normal_match
}

fn find_exact_builtin(name: &str, param_count: usize, builtins: &[Rc<Fun>]) -> Option<Rc<Fun>> {
    builtins
        .iter()
        .find(|f| {
            if f.name.to_lower() != name.to_ascii_lowercase() {
                return false;
            }
            match_arg_count(param_count, &f.args)
        })
        .cloned()
}

fn find_inexact_builtin(name: &str, builtins: &[Rc<Fun>]) -> Option<Rc<Fun>> {
    builtins
        .iter()
        .find(|f| f.name.to_lower() == name.to_ascii_lowercase())
        .cloned()
}
