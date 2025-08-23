use std::rc::Rc;

use regex::Regex;
use uuid::Uuid;

use crate::functions::Depth;
use crate::value::Value;
use crate::{
    access,
    ast::Ast,
    functions::{Fun, FunCall, ParamKind},
    located_ast::LocatedAst,
    location::Location,
    operators::{
        AndOrOperator, AssignmentOperator, CmpOperator, LogicalAndOrOperator, ModShiftOperator,
        UnaryOperator,
    },
    rcsubstring::RcSubString,
    section::Section,
};
use crate::{access::var_kind::VarKind, iterators::dots::Dots};

mod info;
mod kind;
mod origin;
mod returns;
mod tests;
mod undetermined;
pub mod var_kind;

/// Access in the global scope
#[derive(Debug)]
pub struct TopLevel {
    pub origin: Origin,
    pub info: Info,
    pub section: &'static str,
}

/// Access in a function scope
#[derive(Debug)]
pub struct WithinFunction {
    pub info: Info,
    pub var_kind: VarKind,
    pub uuid: Uuid,
    pub origin: Origin,
}

/// Access that is not yet determined to be a `TopLevel` or a `WithinFunction`
/// Returned by `get_accesses`
#[derive(Debug, Clone)]
pub struct Undetermined {
    pub origin: Origin,
    pub info: Info,
    /// Indicates whether the access should be forced to global scope when transforming into a `WithinFunction`
    pub force_global_scope: bool,
    /// Indicates whether the global modifier should be bypassed when transforming into a `WithinFunction`
    pub bypass_global_modifier: bool,
}

#[derive(Debug, Clone)]
pub struct Info {
    pub accessed_as: RcSubString,
    pub location: Location,
    pub kind: Kind,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Read,
    Write { value: Value, potential: bool },
    PassedByRef,
}

/// Origin of the access.
/// Example:
/// ```
/// function foo() (
///     // Access `this.in` (uuid: A)
///     this.in = 10;
/// );
/// function bar() (
///     // Access `hello.in` (uuid: B) (origin: A)
///     hello.foo();
/// );
/// bar();
/// ```
#[derive(Debug, Clone)]
pub enum Origin {
    Global,
    This(OriginDetails),
    Instance(OriginDetails),
    Ref(OriginDetails),
    Undetermined,
}

#[derive(Debug, Clone)]
pub struct OriginDetails {
    uuid: Uuid,
    /// When resolving a navigation, and there were more dots than prefix (i.e. `navigation_reached_global == true`),
    /// in some cases the global scope is replaced by a named variable.
    /// This is the name of the variable that replaces the global scope.
    /// Example:
    /// ```
    /// @init
    /// function nested() (
    ///   this.X = 1234;
    /// );
    /// function inter() (
    ///   this..nested(); // Navigated too far
    /// );
    /// obj1.inter(); // nested.X = 1234
    /// ```
    global_scope_navigation_override: Option<RcSubString>,
}

#[derive(Debug, Clone)]
pub enum ReturnKind {
    Named(RcSubString),
    Value(Value),
}

#[derive(Debug, Clone)]
pub struct Return {
    pub location: Location,
    /// eg. when returning `c > d ? a : b` both a and b are potential
    pub potential: bool,
    pub kind: ReturnKind,
}

impl Origin {
    pub fn global_scope_navigation_override(&self) -> &Option<RcSubString> {
        match self {
            Self::Undetermined | Self::Global => &None,
            Self::This(OriginDetails {
                global_scope_navigation_override,
                ..
            })
            | Self::Instance(OriginDetails {
                global_scope_navigation_override,
                ..
            })
            | Self::Ref(OriginDetails {
                global_scope_navigation_override,
                ..
            }) => global_scope_navigation_override,
        }
    }

    pub fn set_navigation_override_if_none(&mut self, new_override: Option<RcSubString>) {
        let Some(new_override) = new_override else {
            return;
        };
        match self {
            Self::Undetermined | Self::Global => {}
            Self::This(OriginDetails {
                global_scope_navigation_override,
                ..
            })
            | Self::Instance(OriginDetails {
                global_scope_navigation_override,
                ..
            })
            | Self::Ref(OriginDetails {
                global_scope_navigation_override,
                ..
            }) => {
                if global_scope_navigation_override.is_none() {
                    *global_scope_navigation_override = Some(new_override);
                }
            }
        }
    }
}

/// Recursively fills `accesses` with `root`'s accesses and returns potential `Return` values/named
pub fn get_accesses(
    root: &LocatedAst,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
) -> Option<Vec<Return>> {
    match &root.ast {
        Ast::StringIdentifier { value, .. } | Ast::Identifier { value, .. } => Some(vec![Return {
            kind: ReturnKind::Named(value.clone()),
            location: root.location.clone(),
            potential: false,
        }]),
        Ast::If { condition, yes, no } => {
            get_accesses_from_if(condition, accesses, section, yes, no)
        }
        Ast::Compound { expressions, .. } => {
            get_accesses_from_compound(expressions, accesses, section)
        }
        Ast::Assignment { operator, lhs, rhs } => {
            get_accesses_from_assignment(rhs, accesses, section, lhs, operator, root)
        }
        Ast::Add { lhs, rhs }
        | Ast::Sub { lhs, rhs }
        | Ast::Div { lhs, rhs }
        | Ast::Pow { lhs, rhs }
        | Ast::Mul { lhs, rhs }
        | Ast::ModShift { lhs, rhs, .. }
        | Ast::LogicalAndOr { lhs, rhs, .. }
        | Ast::AndOr { lhs, rhs, .. }
        | Ast::Cmp { lhs, rhs, .. } => get_accesses_from_op(rhs, accesses, section, lhs, root),
        Ast::Unary { operand, operator } => {
            get_accesses_from_unary(operand, accesses, section, operator)
        }
        Ast::Loop { count, body } => get_accesses_from_loop(count, accesses, section, body),
        Ast::While { condition, body } => {
            get_accesses_from_while(condition, accesses, section, body)
        }
        Ast::MemoryAccess { rvalue, index } => {
            get_accesses_from_mem_access(index, accesses, section, rvalue)
        }
        Ast::FunCall { uuid, params, .. } => {
            get_accesses_from_fun_call(params, accesses, section, uuid, root)
        }
        Ast::Number(num) => get_accesses_from_number(num, root),
        Ast::Program(p) => get_accesses(p, accesses, section),
        Ast::Void | Ast::String { .. } | Ast::Fun { .. } | Ast::CharLit(_) => None,
        Ast::Arg { .. } | Ast::Unnecessary { .. } | Ast::FunMod { .. } => {
            unreachable!()
        }
    }
}

fn get_accesses_from_number(num: &RcSubString, root: &LocatedAst) -> Option<Vec<Return>> {
    let parsed = num.parse::<f64>();
    parsed.map_or(None, |parsed| {
        Some(vec![Return {
            kind: ReturnKind::Value(Value::Number(parsed)),
            location: root.location.clone(),
            potential: false,
        }])
    })
}

fn get_accesses_from_mem_access(
    index: &Option<Box<LocatedAst>>,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    rvalue: &LocatedAst,
) -> Option<Vec<Return>> {
    if let Some(index) = index {
        let ret_i = get_accesses(index, accesses, section);
        fun_returns_to_read_accesses(&ret_i, accesses);
    }
    let ret_r = get_accesses(rvalue, accesses, section);
    fun_returns_to_read_accesses(&ret_r, accesses);
    None
}

fn get_accesses_from_while(
    condition: &Option<Box<LocatedAst>>,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    body: &Option<Box<LocatedAst>>,
) -> Option<Vec<Return>> {
    if let Some(condition) = condition {
        let cond_ret = get_accesses(condition, accesses, section);
        fun_returns_to_read_accesses(&cond_ret, accesses);
    }
    if let Some(body) = body {
        // "while()" used as rvalue always return 0, no need to read body's returns
        get_accesses(body, accesses, section);
    }
    None
}

fn get_accesses_from_loop(
    count: &LocatedAst,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    body: &LocatedAst,
) -> Option<Vec<Return>> {
    let ret_count = get_accesses(count, accesses, section);
    fun_returns_to_read_accesses(&ret_count, accesses);
    // "loop()" used as rvalue always return 1, no need to read body's returns
    get_accesses(body, accesses, section);
    None
}

fn get_accesses_from_compound(
    exprs: &Vec<LocatedAst>,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
) -> Option<Vec<Return>> {
    let mut last = None;
    for expr in exprs {
        last = get_accesses(expr, accesses, section);
    }
    last
}

fn get_accesses_from_fun_call(
    params: &Option<Vec<LocatedAst>>,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    uuid: &Uuid,
    root: &LocatedAst,
) -> Option<Vec<Return>> {
    let fun_call = section
        .uuid_to_fun_calls
        .get(uuid)
        .expect("FunCall should exist");
    get_accesses_from_params(params, accesses, section, &fun_call.fun);
    let Some(called_fun) = &fun_call.fun else {
        // Function is unknown, nothing more to do here
        return None;
    };
    if called_fun.is_builtin {
        get_accesses_from_builtin_fun_call(accesses, fun_call, called_fun, root);
    } else {
        get_accesses_from_user_fun_call(fun_call, called_fun, accesses);
    }
    if called_fun.scope.returns.is_empty() {
        return None;
    }
    let returns = called_fun.scope.returns.clone();
    Some(returns)
}

fn get_accesses_from_builtin_fun_call(
    accesses: &mut Vec<Undetermined>,
    fun_call: &Rc<FunCall>,
    called_fun: &Rc<Fun>,
    root: &LocatedAst,
) {
    // The called function is a builtin, its definition was never parsed and no scope exist for it.
    // Consider all ref args written and read.
    for (i, arg) in called_fun.args.iter().enumerate() {
        let Some(params) = fun_call.params.get(i) else {
            // Not enough params
            continue;
        };
        if !arg.is_ref {
            continue;
        }
        for param in params {
            let ParamKind::Identifier { name } = &param.kind else {
                // Not a named param
                continue;
            };
            let name_rc = RcSubString::from_str(name);
            accesses.push(Undetermined {
                origin: Origin::Undetermined,
                info: Info {
                    location: param.location.clone(),
                    accessed_as: name_rc.clone(),
                    kind: Kind::Read,
                },
                force_global_scope: false,
                bypass_global_modifier: false,
            });
            accesses.push(Undetermined {
                origin: Origin::Undetermined,
                info: Info {
                    location: param.location.clone(),
                    accessed_as: name_rc,
                    kind: Kind::Write {
                        value: Value::Unknown,
                        potential: false,
                    },
                },
                force_global_scope: false,
                bypass_global_modifier: false,
            });
        }
    }

    // Try to register reads for sprintf format arg
    get_accesses_from_sprintf(called_fun, fun_call, accesses, root);
}

/// Gets accesses from a function call, for a user-defined function (i.e. not a builtin).
fn get_accesses_from_user_fun_call(
    fun_call: &Rc<FunCall>,
    called_fun: &Rc<Fun>,
    accesses: &mut Vec<Undetermined>,
) {
    for within_fun in &called_fun.scope.accesses {
        match &within_fun.var_kind {
            VarKind::This { suffix } => {
                get_access_from_within_this(
                    within_fun,
                    fun_call,
                    called_fun,
                    suffix.as_ref(),
                    accesses,
                );
            }

            VarKind::Instance { .. } => {
                // An instance access is equivalent to a "this" access
                let suffix = Some(&within_fun.info.accessed_as);
                get_access_from_within_this(within_fun, fun_call, called_fun, suffix, accesses);
            }

            VarKind::RefArg { suffix, arg_index } => {
                get_access_from_ref_arg(within_fun, fun_call, *arg_index, suffix, accesses);
            }

            VarKind::Global { accessible: true } => accesses.push(Undetermined {
                origin: Origin::Global,
                info: within_fun.info.clone(),
                force_global_scope: true,
                bypass_global_modifier: true,
            }),

            // Accesses of these VarKind do not propagate to the outer scope
            VarKind::Arg { .. }
            | VarKind::Local
            | VarKind::Global { accessible: false }
            | VarKind::TempString => (),
        }
    }
}

fn get_access_from_ref_arg(
    within_function: &WithinFunction,
    fun_call: &Rc<FunCall>,
    arg_index: usize,
    suffix: &RcSubString,
    accesses: &mut Vec<Undetermined>,
) {
    let names = fun_call.ref_arg_to_accessed_as(suffix.as_str(), arg_index);
    let Some(names) = names else {
        // Not enough params
        return;
    };
    for name in names {
        accesses.push(Undetermined {
            origin: within_function.origin.or_if_undetermined(|| {
                Origin::Ref(OriginDetails {
                    uuid: within_function.uuid,
                    global_scope_navigation_override: None,
                })
            }),
            info: Info {
                accessed_as: RcSubString::from_str(&name),
                location: fun_call.location.clone(),
                kind: within_function.info.kind.clone(),
            },
            force_global_scope: false,
            bypass_global_modifier: false,
        });
    }
}

fn get_access_from_within_this(
    within_function: &WithinFunction,
    fun_call: &Rc<FunCall>,
    called_fun: &Rc<Fun>,
    suffix: Option<&RcSubString>,
    accesses: &mut Vec<Undetermined>,
) {
    let origin = &within_function.origin;
    let nested_this_access = resolve_within_this_access(fun_call, called_fun, suffix);
    let accessed_as = if nested_this_access.navigation_reached_global {
        if let Some(navigation_override) = origin.global_scope_navigation_override() {
            format!("{}.{}", navigation_override, nested_this_access.accessed_as)
        } else {
            nested_this_access.accessed_as
        }
    } else {
        nested_this_access.accessed_as
    };
    let origin = if matches!(origin, Origin::Undetermined) {
        Origin::This(OriginDetails {
            uuid: within_function.uuid,
            global_scope_navigation_override: nested_this_access.navigation_override,
        })
    } else {
        let mut origin = origin.clone();
        origin.set_navigation_override_if_none(nested_this_access.navigation_override);
        origin
    };

    // Variable doesn't need to be further resolved in the outer function
    let force_global_scope =
        !fun_call.name_matches_instance_arg && !fun_call.is_nested_and_has_prefix();

    accesses.push(Undetermined {
        origin,
        force_global_scope,
        bypass_global_modifier: force_global_scope || fun_call.name_matches_global_arg,
        info: Info {
            accessed_as: RcSubString::from_str(&accessed_as),
            location: fun_call.location.clone(),
            kind: within_function.info.kind.clone(),
        },
    });
}

/// Classification of a function call prefix.
///
/// Note: `FunCallPrefix` doesn't have a `None` variant because if a function call doesn't have a prefix,
/// the function name is used instead.
#[derive(Debug)]
enum FunCallPrefix {
    /// `this` followed by one dot, and a potential suffix e.g `this.foo()` or `this.foo.bar()`
    /// Note: this implies that the function call is nested (e.g. not a top level function call).
    This {
        /// What comes after "this" and the dots.
        /// Examples:
        /// - `this.foo()` -> `None`
        /// - `this.inner.foo()` -> `Some("inner")`
        this_suffix: Option<RcSubString>,

        /// Full function call prefix without the dot
        /// Examples:
        /// - `this.foo()` -> `"this"`
        /// - `this.inner.foo()` -> `"this.inner"`
        prefix: RcSubString,
    },

    /// `this` followed by `dot_count` dots (two or more) and a potential suffix e.g `this..foo()` or `this..foo.bar()`.
    /// Note: this implies that the function call is nested (e.g. not a top level function call).
    ThisNavigation {
        /// What comes after "this" and the dots.
        /// Examples:
        /// - `this..foo()` -> `None`
        /// - `this..inner.foo()` -> `Some("inner")`
        this_suffix: Option<RcSubString>,

        /// What comes before `this_suffix`
        /// Examples:
        /// - `this..foo()` -> `"this.."`
        /// - `this..inner.foo()` -> `"this.."`
        this_prefix: RcSubString,

        /// Number of dots after the "this" prefix.
        /// Note:
        /// - if `this_suffix` is `None`, `dot_count == 1` means "go back one level" (e.g. `this..foo()`)
        /// - if `this_suffix` is `Some`, `dot_count == 2` means "go back one level" (e.g. `this..inner.foo()`)
        ///
        /// Note: `dot_count` can't be `1` if `this_suffix` is `Some`.
        /// In that case, the prefix wouldn't be a `ThisNavigation` but rather a `This`. (e.g. `this.obj.foo()`)
        dot_count: usize,

        /// Full function call prefix without the dot
        /// Examples:
        /// - `this..foo()` -> `"this."`
        /// - `this.inner.foo()` -> `"this.inner"`
        /// - `this..inner.foo()` -> `"this..inner"`
        prefix: RcSubString,
    },

    Other(RcSubString),
}

/// Classification of a `this` access suffix.
#[derive(Debug)]
enum ThisAccessSuffix {
    /// `this` followed by `dot_count` dots (two or more) and a suffix e.g `this..foo` or `this..foo.bar`
    Navigation {
        suffix: RcSubString,
        suffix_after_dots: RcSubString,
        dot_count: usize,
    },
    /// `this` followed by a single dot and a suffix e.g `this.foo` or `this.foo.bar`
    Normal(RcSubString),
    /// `this` without any suffix e.g `this`
    None,
}

fn classify_this_access_suffix(suffix: Option<&RcSubString>) -> ThisAccessSuffix {
    let Some(suffix) = suffix else {
        return ThisAccessSuffix::None;
    };
    let dot_count = suffix.chars().take_while(|c| *c == '.').count();
    if dot_count > 0 {
        ThisAccessSuffix::Navigation {
            suffix: suffix.clone(),
            suffix_after_dots: suffix.substr(dot_count..),
            dot_count,
        }
    } else {
        ThisAccessSuffix::Normal(suffix.clone())
    }
}

fn classify_fun_call_prefix(fun_call: &FunCall) -> FunCallPrefix {
    let prefix = RcSubString::from_str(&fun_call.get_prefix());
    if fun_call.prefix.is_none() {
        // Function call doesn't have a prefix, use callee name
        // or if callee as a dot in its name, use the part before the dot as prefix.
        return FunCallPrefix::Other(prefix);
    }
    if matches!(fun_call.depth, Depth::TopLevel) {
        // Function call is top level, use prefix as is.
        return FunCallPrefix::Other(prefix);
    }
    let prefix_lower = prefix.to_lower();
    if prefix_lower == "this" {
        // Prefix is "this" e.g. this.bar()
        return FunCallPrefix::This {
            this_suffix: None,
            prefix: prefix.clone(),
        };
    }
    if !prefix_lower.starts_with("this.") {
        // Not a "this" prefix
        return FunCallPrefix::Other(prefix.clone());
    }
    // Prefix is of the form "this." with one or more dots and a potential suffix
    let suffix = prefix.substr("this".len()..);
    let dot_count = suffix.chars().take_while(|c| *c == '.').count();
    if dot_count == suffix.len() {
        // Only dots, no suffix
        FunCallPrefix::ThisNavigation {
            dot_count,
            this_suffix: None,
            prefix: prefix.clone(),
            this_prefix: prefix.clone(),
        }
    } else {
        // Dot(s) and suffix
        let suffix_after_dots = suffix.substr(dot_count..);
        if dot_count == 1 {
            // One dot and a suffix e.g. this.obj.foo();
            FunCallPrefix::This {
                this_suffix: Some(suffix_after_dots),
                prefix: prefix.clone(),
            }
        } else {
            FunCallPrefix::ThisNavigation {
                this_prefix: prefix.substr(..prefix.len() - suffix_after_dots.len()),
                this_suffix: Some(suffix_after_dots),
                dot_count,
                prefix: prefix.clone(),
            }
        }
    }
}

struct NestedThisAccess {
    accessed_as: String,
    navigation_override: Option<RcSubString>,
    navigation_reached_global: bool,
}

#[allow(clippy::too_many_lines)]
fn resolve_within_this_access(
    fun_call: &FunCall,
    called_fun: &Fun,
    initial_access_suffix: Option<&RcSubString>,
) -> NestedThisAccess {
    if let Some(access) = resolve_instance_arg_match(fun_call, initial_access_suffix) {
        return access;
    }

    let fun_call_prefix = classify_fun_call_prefix(fun_call);
    let access_suffix = classify_this_access_suffix(initial_access_suffix);

    match (fun_call_prefix, access_suffix) {
        // Example:
        // function foo() (
        //     this.suffix = 10;
        // );
        // function fun() (
        //     this..foo();
        //     // or
        //     this..this_suffix.foo();
        // );
        (
            FunCallPrefix::ThisNavigation {
                prefix,
                this_suffix,
                ..
            },
            ThisAccessSuffix::Normal(suffix),
        ) => {
            let navigation_override =
                this_suffix.map_or_else(|| Some(called_fun.name.clone()), |_| None);
            NestedThisAccess {
                accessed_as: format!("{prefix}.{suffix}"),
                navigation_override,
                navigation_reached_global: false,
            }
        }

        // Example:
        // function foo() (
        //     this..access_suffix = 10;
        // );
        // function fun() (
        //     this..foo();
        //     // or
        //     this..fun_call_this_suffix.foo();
        // );
        (
            FunCallPrefix::ThisNavigation {
                prefix,
                this_suffix: fun_call_this_suffix,
                this_prefix,
                ..
            },
            ThisAccessSuffix::Navigation {
                suffix: access_suffix,
                dot_count,
                ..
            },
        ) => {
            let accessed_as = get_accessed_as(
                prefix.as_str(),
                fun_call_this_suffix.as_ref().map(RcSubString::as_str),
                this_prefix.as_str(),
                access_suffix.as_str(),
                dot_count,
            );
            NestedThisAccess {
                accessed_as,
                navigation_override: None,
                navigation_reached_global: false,
            }
        }

        // Example:
        // function foo() (
        //     this..access_suffix = 10;
        // );
        // function fun() (
        //     this.foo();
        //     // or
        //     this.fun_call_this_suffix.foo();
        // );
        // obj.fun();
        (
            FunCallPrefix::This {
                this_suffix: fun_call_this_suffix,
                ..
            },
            ThisAccessSuffix::Navigation {
                dot_count,
                suffix: access_suffix,
                ..
            },
        ) => {
            let accessed_as = get_accessed_as(
                "this",
                fun_call_this_suffix.as_ref().map(RcSubString::as_str),
                "this.",
                access_suffix.as_str(),
                dot_count,
            );
            NestedThisAccess {
                accessed_as,
                navigation_override: None,
                navigation_reached_global: false,
            }
        }

        // Example:
        // function foo() (
        //     this.suffix = 10;
        // );
        // function fun() (
        //     this.foo();
        //     // or
        //     this.this_suffix.foo();
        // );
        // obj.fun();
        (FunCallPrefix::This { this_suffix, .. }, ThisAccessSuffix::Normal(suffix)) => {
            let accessed_as = match (this_suffix, suffix) {
                (Some(this_suffix), suffix) if suffix.as_str() == "" => {
                    format!("this.{this_suffix}")
                }
                (Some(this_suffix), suffix) => {
                    format!("this.{this_suffix}.{suffix}")
                }
                (None, suffix) if suffix.as_str() == "" => "this".into(),
                (None, suffix) => {
                    format!("this.{suffix}")
                }
            };
            NestedThisAccess {
                accessed_as,
                navigation_override: None,
                navigation_reached_global: false,
            }
        }

        // Example:
        // function foo() (
        //     this..access_suffix = 10;
        // );
        // function fun() (
        //     prefix.foo();
        // );
        (
            FunCallPrefix::Other(prefix),
            ThisAccessSuffix::Navigation {
                dot_count,
                suffix_after_dots,
                ..
            },
        ) => {
            let ResolvedNavigation {
                name,
                reached_global,
            } = resolve_navigation(&prefix, dot_count, &suffix_after_dots);
            NestedThisAccess {
                accessed_as: name,
                navigation_override: None,
                navigation_reached_global: reached_global,
            }
        }

        // Example:
        // function foo() (
        //     this.suffix = 10;
        // );
        // function fun() (
        //     prefix.foo();
        // );
        (FunCallPrefix::Other(prefix), ThisAccessSuffix::Normal(suffix)) => NestedThisAccess {
            accessed_as: if suffix.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}.{suffix}")
            },
            navigation_override: None,
            navigation_reached_global: false,
        },

        // Example:
        // function foo() (
        //     this = 10;
        // );
        // function fun() (
        //     this..foo();
        //     // or
        //     this..this_suffix.foo();
        // );
        // obj.fun();
        (
            FunCallPrefix::ThisNavigation {
                prefix,
                this_suffix,
                ..
            },
            ThisAccessSuffix::None,
        ) => {
            let navigation_override =
                this_suffix.map_or_else(|| Some(called_fun.name.clone()), |_| None);
            NestedThisAccess {
                accessed_as: prefix.to_string(),
                navigation_override,
                navigation_reached_global: false,
            }
        }

        // Example:
        // function foo() (
        //     this = 10;
        // );
        // function fun() (
        //     prefix.foo();
        // );
        // obj.fun();
        (FunCallPrefix::Other(prefix), ThisAccessSuffix::None)
        // Example:
        // function foo() (
        //     this = 10;
        // );
        // function fun() (
        //     this.foo();
        // );
        // obj.fun();
        | (FunCallPrefix::This { prefix, .. }, ThisAccessSuffix::None) => NestedThisAccess {
            accessed_as: prefix.to_string(),
            navigation_override: None,
            navigation_reached_global: false,
        },
    }
}

fn get_accessed_as(
    prefix: &str,
    fun_call_this_suffix: Option<&str>,
    this_prefix: &str,
    access_suffix: &str,
    dot_count: usize,
) -> String {
    let Some(fun_call_this_suffix) = fun_call_this_suffix else {
        // No `this_suffix`, just append the access suffix to the prefix
        return format!("{prefix}.{access_suffix}");
    };
    let dot_positions = Dots::new(fun_call_this_suffix).collect::<Vec<_>>();
    if dot_positions.is_empty() {
        // No dots in the `fun_call_this_suffix`, so we can only navigate one level up
        let new_access_suffix = &access_suffix[1..];
        format!("{this_prefix}{new_access_suffix}")
    } else if dot_count <= dot_positions.len() {
        // We know dot_count can't be 0 here because the suffix is a navigation
        let new_this_suffix_end = dot_positions[dot_positions.len() - dot_count];
        let new_this_suffix = &fun_call_this_suffix[..new_this_suffix_end];
        let new_access_suffix = &access_suffix[dot_count..];
        format!("{this_prefix}{new_this_suffix}.{new_access_suffix}")
    } else {
        // Not enough splits in `this_suffix` to navigate that many levels up.
        // Note: `+ 1` here because there's one more split than dot (e.g. `foo.bar` has 2 splits but 1 dot)
        let new_access_suffix_start = dot_positions.len() + 1;
        let new_access_suffix = &access_suffix[new_access_suffix_start..];
        format!("{this_prefix}{new_access_suffix}")
    }
}

/// See `FunCall::name_matches_instance_arg`
fn resolve_instance_arg_match(
    fun_call: &FunCall,
    initial_access_suffix: Option<&RcSubString>,
) -> Option<NestedThisAccess> {
    if !fun_call.name_matches_instance_arg {
        return None;
    }
    let accessed_as = match (fun_call.prefix.as_ref(), initial_access_suffix) {
        (Some(prefix), Some(suffix)) => {
            format!("this.{prefix}.{suffix}")
        }
        (Some(prefix), None) => prefix.to_string(),
        (None, Some(suffix)) => format!("this.{suffix}"),
        (None, None) => "this".into(),
    };
    Some(NestedThisAccess {
        accessed_as,
        navigation_override: None,
        navigation_reached_global: false,
    })
}

struct ResolvedNavigation {
    reached_global: bool,
    name: String,
}

fn resolve_navigation(
    fun_call_prefix: &str,
    dot_count: usize,
    suffix: &RcSubString,
) -> ResolvedNavigation {
    let prefix_dots = Dots::new(fun_call_prefix).collect::<Vec<_>>();
    // 0 `dot_count` i.e. `this.foo` in `object.inner.fun()` resolves to `object.inner.foo`
    // 1 `dot_count` i.e. `this..foo` in `object.inner.fun()` resolves to `object.foo`
    assert_ne!(
        dot_count, 0,
        "suffix.starts_with('.') so dot_count should be at least 1"
    );
    if dot_count <= prefix_dots.len() {
        let new_prefix_end = prefix_dots[prefix_dots.len() - dot_count];
        let new_prefix = &fun_call_prefix[..new_prefix_end];
        ResolvedNavigation {
            reached_global: false,
            name: format!("{new_prefix}.{suffix}"),
        }
    } else {
        ResolvedNavigation {
            reached_global: true,
            name: suffix.to_string(),
        }
    }
}

fn get_accesses_from_sprintf(
    called_fun: &Rc<Fun>,
    fun_call: &Rc<FunCall>,
    accesses: &mut Vec<Undetermined>,
    root: &LocatedAst,
) {
    if called_fun.name.as_str() != "sprintf" || fun_call.params.get(1).is_none() {
        return;
    }
    let formats = &fun_call.params[1];
    // Take care of "%{var}" in sprintf format string
    // This matches any non-empty "%{}" so there could be false positives
    // But for now it's outside the scope of this project to create a precise parser for C style format strings
    // i.e. "%{hello}d" will read hello
    // but "%{hello}" will read hello when it shouldn't
    let reg = Regex::new(r"%\{([^}]+?)}").unwrap();
    for format in formats {
        if let ParamKind::StringValue { value } = &format.kind {
            for cap in reg.captures_iter(value) {
                // Note: variables read in sprintf format string are always global (and bypass global() modifiers)
                // Note: `this` won't resolve, but we can still access "instance" variables by calling them by their global name,
                //        but there's no need to mark them as instance/ref because they will always point to the same "object",
                //        and we're only interested in marking instance/ref accesses that access multiple global variables.
                accesses.push(Undetermined {
                    origin: Origin::Global,
                    info: Info {
                        accessed_as: RcSubString::from_str(&cap[1]),
                        location: root.location.clone(),
                        kind: access::Kind::Read,
                    },
                    force_global_scope: true,
                    bypass_global_modifier: true,
                });
            }
        }
    }
}

fn get_accesses_from_params(
    params: &Option<Vec<LocatedAst>>,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    fun: &Option<Rc<Fun>>,
) {
    let Some(params) = params else {
        return;
    };
    // We check read and writes of params here
    // For things like `fun(b = a + 1)`
    // That means that even if the associated arg is not read inside the function, we count the param as read
    for (param_idx, param) in params.iter().enumerate() {
        let rets = get_accesses(param, accesses, section);
        let Some(rets) = rets else {
            // No returns
            continue;
        };
        let is_ref = fun
            .as_ref()
            .and_then(|fun| fun.get_arg(param_idx))
            .is_some_and(|arg| arg.is_ref);
        if is_ref {
            // Don't read the ref args here. They will be read only through the ref arg inside the function
            // It's done this way to avoid this situation:
            // Here, if passing `a` to `foo` by ref is considered a read,
            // the linter looks for writes to `a`, can't find any, so it reports an issue.
            // ```
            // function foo(bar*) ( _ = bar.baz; );
            // foo(a); // a is never written
            // ```
            for ret in rets {
                if let Some(passed_by_ref) = ret.to_passed_by_ref() {
                    accesses.push(passed_by_ref);
                }
            }
        } else {
            for ret in rets {
                if let Some(read) = ret.to_read() {
                    accesses.push(read);
                }
            }
        }
    }
}

fn get_accesses_from_unary(
    operand: &LocatedAst,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    operator: &UnaryOperator,
) -> Option<Vec<Return>> {
    let ret = get_accesses(operand, accesses, section);
    fun_returns_to_read_accesses(&ret, accesses);
    if let Some(ret) = &ret {
        let mut new_value = None;
        let mut new_location = None;
        let mut new_potential = None;
        match &ret[..] {
            [
                Return {
                    kind: ReturnKind::Value(Value::Number(value)),
                    location,
                    potential,
                },
            ] => {
                new_value = Some(*value);
                new_location = Some(location.clone());
                new_potential = Some(*potential);
            }
            [
                Return {
                    kind: ReturnKind::Named(name),
                    location,
                    potential,
                },
            ] => {
                if let Value::Number(value) = get_value_from_previous_accesses(name, accesses) {
                    new_value = Some(value);
                    new_location = Some(location.clone());
                    new_potential = Some(*potential);
                }
            }
            _ => (),
        }
        if let (Some(new_value), Some(new_location), Some(new_potential)) =
            (new_value, new_location, new_potential)
        {
            let result = get_unary_result(operator, new_value);
            return Some(vec![Return {
                kind: ReturnKind::Value(result),
                location: new_location,
                potential: new_potential,
            }]);
        }
    }
    None
}

fn get_accesses_from_op(
    rhs: &LocatedAst,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    lhs: &LocatedAst,
    root: &LocatedAst,
) -> Option<Vec<Return>> {
    let rets_rhs = get_accesses(rhs, accesses, section);
    let rets_lhs = get_accesses(lhs, accesses, section);

    let val_lhs = returns_to_value(accesses, &rets_lhs);
    let val_rhs = returns_to_value(accesses, &rets_rhs);

    fun_returns_to_read_accesses(&rets_rhs, accesses);
    fun_returns_to_read_accesses(&rets_lhs, accesses);

    match (val_lhs, val_rhs) {
        (Value::Number(a), Value::Number(b)) => Some(vec![Return {
            kind: ReturnKind::Value(get_op_result(&root.ast, a, b)),
            location: root.location.clone(),
            potential: false,
        }]),
        _ => None,
    }
}

fn returns_to_value(accesses: &Vec<Undetermined>, returns: &Option<Vec<Return>>) -> Value {
    returns
        .as_ref()
        .map_or(Value::Unknown, |rets_a| match &rets_a[..] {
            [r] => match &r.kind {
                ReturnKind::Named(name) => get_value_from_previous_accesses(name, accesses),
                ReturnKind::Value(value) => value.clone(),
            },
            _ => Value::Unknown,
        })
}

fn get_accesses_from_assignment(
    rhs: &LocatedAst,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    lhs: &LocatedAst,
    operator: &AssignmentOperator,
    root: &LocatedAst,
) -> Option<Vec<Return>> {
    let ret_rhs = get_accesses(rhs, accesses, section);
    let rhs_value = ret_rhs
        .as_ref()
        .map_or(Value::Unknown, |ret_b| match &ret_b[..] {
            [x] => {
                if x.potential {
                    Value::Unknown
                } else {
                    match &x.kind {
                        ReturnKind::Value(value) => value.clone(),
                        ReturnKind::Named(name) => {
                            // Try to find the value in current accesses, but no further
                            // (There could still be concurrent writes in @gfx, but we'll ignore that for now)
                            //IDEA don't ignore that :)
                            get_value_from_previous_accesses(name, accesses)
                        }
                    }
                }
            }
            _ => Value::Unknown,
        });
    fun_returns_to_read_accesses(&ret_rhs, accesses);
    let ret_lhs = get_accesses(lhs, accesses, section)?;
    for ret in &ret_lhs {
        match &ret.kind {
            ReturnKind::Named(name) => {
                // Assigning to a variable
                let lhs_value = get_value_from_previous_accesses(name, accesses);
                let value = get_assignment_value(operator, &lhs_value, &rhs_value);

                // Read `lhs` if operator is not "="
                if !matches!(operator, AssignmentOperator::Assign)
                    && let Some(read) = ret.to_read()
                {
                    accesses.push(read);
                }

                if let Some(write) = ret.to_write(value.clone()) {
                    accesses.push(write);
                }
            }
            ReturnKind::Value(value) => {
                // Assigning to a value. Return a value but obviously don't add read/write access for lhs
                // Note: JSFX doesn't allow `1 += 2` but allows `(1) += 2`, hence this
                let value = get_assignment_value(operator, value, &rhs_value);
                return Some(vec![Return {
                    kind: ReturnKind::Value(value),
                    location: root.location.clone(),
                    potential: ret.potential,
                }]);
            }
        }
    }
    Some(ret_lhs)
}

fn get_accesses_from_if(
    condition: &LocatedAst,
    accesses: &mut Vec<Undetermined>,
    section: &Section,
    yes: &Option<Box<LocatedAst>>,
    no: &Option<Box<LocatedAst>>,
) -> Option<Vec<Return>> {
    let cond_rets = get_accesses(condition, accesses, section);
    fun_returns_to_read_accesses(&cond_rets, accesses);
    let mut yes_access = None;
    let mut no_access = None;
    let mut yes_ret = None;
    let mut no_ret = None;
    if let Some(yes) = yes {
        let mut yes_vec = Vec::new();
        yes_ret = get_accesses(yes, &mut yes_vec, section);
        yes_access = Some(yes_vec);
    }
    if let Some(no) = no {
        let mut no_vec = Vec::new();
        no_ret = get_accesses(no, &mut no_vec, section);
        no_access = Some(no_vec);
    }
    let mut all_ret = Vec::new();
    match (yes_ret, no_ret) {
        (Some(yes), None) => {
            for ret in yes {
                all_ret.push(ret.into_potential());
            }
        }
        (None, Some(no)) => {
            for ret in no {
                all_ret.push(ret.into_potential());
            }
        }
        (Some(mut yes), Some(no)) => {
            // yes and no returns are identical only if all are the same
            let mut same = true;
            let first = &yes[0];
            for ret in &yes {
                if ret.is_equivalent(first) {
                    same = false;
                    break;
                }
            }
            for ret in &no {
                if ret.is_equivalent(first) {
                    same = false;
                    break;
                }
            }
            if same {
                all_ret.append(&mut yes);
            } else {
                all_ret.extend(yes.into_iter().map(Return::into_potential));
                all_ret.extend(no.into_iter().map(Return::into_potential));
            }
        }
        (None, None) => (),
    }
    match (yes_access, no_access) {
        (Some(yes), None) => {
            for acc in yes {
                accesses.push(acc.to_potential());
            }
        }
        (None, Some(no)) => {
            for acc in no {
                accesses.push(acc.to_potential());
            }
        }
        (Some(yes), Some(no)) => {
            for acc in yes {
                accesses.push(acc.to_potential());
            }
            for acc in no {
                accesses.push(acc.to_potential());
            }
        }
        (None, None) => (),
    }
    if all_ret.is_empty() {
        None
    } else {
        Some(all_ret)
    }
}

#[allow(clippy::cast_possible_truncation)]
fn get_assignment_value(operator: &AssignmentOperator, val_lhs: &Value, val_rhs: &Value) -> Value {
    let Value::Number(val_rhs) = val_rhs else {
        return Value::Unknown;
    };
    // `=` only need to know `rhs`
    if matches!(operator, AssignmentOperator::Assign) {
        return Value::Number(*val_rhs);
    }
    let Value::Number(val_lhs) = val_lhs else {
        return Value::Unknown;
    };
    // Some operator convert there operands to integers
    let left_int = val_lhs.round() as i32;
    let right_int = val_rhs.round() as i32;

    match operator {
        // Already done above
        AssignmentOperator::Assign => unreachable!(),
        AssignmentOperator::Add => Value::Number(val_lhs + val_rhs),
        AssignmentOperator::Sub => Value::Number(val_lhs - val_rhs),
        AssignmentOperator::Mul => Value::Number(val_lhs * val_rhs),
        AssignmentOperator::Div => Value::Number(val_lhs / val_rhs),
        AssignmentOperator::Mod => Value::Number(f64::from(left_int.abs() % right_int.abs())),
        AssignmentOperator::Or => Value::Number(f64::from(left_int | right_int)),
        AssignmentOperator::And => Value::Number(f64::from(left_int & right_int)),
        AssignmentOperator::Xor => Value::Number(f64::from(left_int ^ right_int)),
        AssignmentOperator::Pow => Value::Number(val_lhs.powf(*val_rhs)),
    }
}

#[allow(clippy::cast_possible_truncation)]
fn get_op_result(ast: &Ast, val_lhs: f64, val_rhs: f64) -> Value {
    // Some operator convert there operands to integers
    let left_int = val_lhs.round() as i32;
    let right_int = val_rhs.round() as i32;
    match ast {
        Ast::Add { .. } => Value::Number(val_lhs + val_rhs),
        Ast::Sub { .. } => Value::Number(val_lhs - val_rhs),
        Ast::Div { .. } => Value::Number(val_lhs / val_rhs),
        Ast::Mul { .. } => Value::Number(val_lhs * val_rhs),
        Ast::ModShift { operator, .. } => match operator {
            ModShiftOperator::Left => Value::Number(f64::from(left_int << right_int)),
            ModShiftOperator::Right => Value::Number(f64::from(left_int >> right_int)),
            ModShiftOperator::Mod => {
                Value::Number(f64::from(val_lhs.abs() as i32 % val_rhs.abs() as i32))
            }
        },
        Ast::LogicalAndOr { operator, .. } => match operator {
            LogicalAndOrOperator::And => Value::Number(if val_lhs != 0.0 && val_rhs != 0.0 {
                1.0
            } else {
                0.0
            }),
            LogicalAndOrOperator::Or => Value::Number(if val_lhs != 0.0 || val_rhs != 0.0 {
                1.0
            } else {
                0.0
            }),
        },
        Ast::AndOr { operator, .. } => match operator {
            AndOrOperator::And => Value::Number(f64::from(val_lhs as i32 & val_rhs as i32)),
            AndOrOperator::Or => Value::Number(f64::from(val_lhs as i32 | val_rhs as i32)),
            AndOrOperator::Xor => Value::Number(f64::from(val_lhs as i32 ^ val_rhs as i32)),
        },
        Ast::Cmp { operator, .. } => match operator {
            CmpOperator::Eq => Value::Number(if (val_lhs - val_rhs).abs() < 0.00001 {
                1.0
            } else {
                0.0
            }),
            CmpOperator::Ne => Value::Number(if (val_lhs - val_rhs).abs() >= 0.00001 {
                1.0
            } else {
                0.0
            }),
            CmpOperator::Gt => Value::Number(if val_lhs > val_rhs { 1.0 } else { 0.0 }),
            CmpOperator::Gte => Value::Number(if val_lhs >= val_rhs { 1.0 } else { 0.0 }),
            CmpOperator::Lt => Value::Number(if val_lhs < val_rhs { 1.0 } else { 0.0 }),
            CmpOperator::Lte => Value::Number(if val_lhs <= val_rhs { 1.0 } else { 0.0 }),
            CmpOperator::ExactEq => Value::Number(if approx(val_lhs, val_rhs) { 1.0 } else { 0.0 }),
            CmpOperator::ExactNe => Value::Number(if approx(val_lhs, val_rhs) { 0.0 } else { 1.0 }),
        },
        _ => Value::Unknown,
    }
}

fn get_unary_result(operator: &UnaryOperator, val: f64) -> Value {
    match operator {
        UnaryOperator::Not => Value::Number(if val == 0.0 { 1.0 } else { 0.0 }),
        UnaryOperator::Neg => Value::Number(-val),
        UnaryOperator::Pos => Value::Number(val),
    }
}

fn get_value_from_previous_accesses(name: &RcSubString, accesses: &Vec<Undetermined>) -> Value {
    let mut last = None;
    for access in accesses {
        if let Undetermined {
            info:
                Info {
                    accessed_as: access_name,
                    kind: access::Kind::Write { value, potential },
                    ..
                },
            ..
        } = access
            && name.to_lower() == access_name.to_lower()
        {
            last = if *potential { None } else { Some(value) }
        }
    }
    last.cloned().unwrap_or(Value::Unknown)
}

pub fn fun_returns_to_read_accesses(rets: &Option<Vec<Return>>, access: &mut Vec<Undetermined>) {
    if let Some(rets) = rets {
        for r in rets {
            if let Some(read) = r.to_read() {
                access.push(read);
            }
        }
    }
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < f64::EPSILON
}
