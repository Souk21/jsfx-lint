use std::rc::Rc;

use crate::{
    functions::{ArgIndex, FunCall, ParamKind},
    iterators::dots::Dots,
    located_ast::LocatedAst,
    rcsubstring::RcSubString,
    section::Section,
};

use super::{Depth, Fun, ModifierKind};

impl FunCall {
    /// Instance reads and writes need to consider the prefix if it exists, else it uses the function name.
    /// `this` reads (and writes) are instance reads but without suffix.
    /// Otherwise, returns `Option<(prefix, var_name)>`
    /// Example:
    /// ```
    ///   function foo() ( this.bar = 1; this = 1; );
    ///   baz.foo() // writes to baz.bar and baz
    ///   foo() // writes to foo.bar and foo
    /// ```
    pub fn instance_var_to_prefixed(&self, variable_name: &str) -> (RcSubString, String) {
        let prefix = self.get_prefix();
        let var_name = format!("{prefix}.{variable_name}");
        (prefix, var_name)
    }

    /// Returns the prefix of the function call, if it exists, else the function name.
    /// Example:
    /// ```
    /// bar.foo() // prefix: bar
    /// this.foo() // prefix: this
    /// foo() // prefix: foo
    /// baz.foo.bar() // prefix: baz.foo
    /// ```
    pub fn get_prefix(&self) -> RcSubString {
        Dots::new(self.accessed_as.as_str()).last().map_or_else(
            || self.accessed_as.clone(),
            |last_dot| self.accessed_as.substr(..last_dot),
        )
    }

    /// Returns the called function name without the prefix (keeping call-site casing)
    pub fn fun_name(&self) -> &str {
        self.prefix.as_ref().map_or_else(
            || self.accessed_as.as_str(),
            |prefix| {
                self.accessed_as
                    .as_str()
                    .strip_prefix(&format!("{prefix}."))
                    .expect("Function name doesn't start with prefix")
            },
        )
    }

    /// Returns whether the function call is nested and has a prefix
    /// Useful when checking if accesses from a function needs to be further resolved
    pub const fn is_nested_and_has_prefix(&self) -> bool {
        matches!(self.depth, Depth::Nested { .. }) && self.prefix.is_some()
    }

    /// Returns all the variables that are accessed when the reference argument is accessed.
    pub fn ref_arg_to_accessed_as(&self, suffix: &str, arg_index: ArgIndex) -> Option<Vec<String>> {
        let mut var_names = Vec::new();
        let called_fun = self.fun.as_ref()?;
        let arg = called_fun.args.get(arg_index)?;
        if arg_index >= self.params.len() {
            // Not enough params
            return None;
        }
        // If arg is "...", access all params with `index >= arg_index`
        if arg.name.as_str() == "..." {
            for params in &self.params[arg_index..] {
                for param in params {
                    if let ParamKind::Identifier { name: param_name } = &param.kind {
                        var_names.push(format!("{param_name}{suffix}"));
                    }
                }
            }
            return Some(var_names);
        }

        let params = self.params.get(arg_index).expect("Param not found");
        for param in params {
            let ParamKind::Identifier { name: param_name } = &param.kind else {
                // param is a value and not a namespace, and this error was reported earlier
                continue;
            };
            var_names.push(format!("{param_name}{suffix}"));
        }
        Some(var_names)
    }

    pub fn start_setup(
        params: &Option<Vec<LocatedAst>>,
        id: &LocatedAst,
        section: &Section,
        init_section: &Option<Section>,
        builtins: &[Rc<Fun>],
        is_in_fn: bool,
        uuid: &uuid::Uuid,
    ) -> Self {
        let param_count = params.as_ref().map_or(0, Vec::len);
        let name = id
            .ast
            .identifier()
            .expect("Non identifier in identifier position");

        let super::MatchingFun {
            fun: called_fun,
            prefix,
        } = super::find_matching_function(section, name, param_count, init_section, builtins);

        let params_vec = called_fun
            .as_ref()
            .map_or_else(Vec::new, |fun| super::params::collect(params, fun));

        let depth = if is_in_fn {
            Depth::Undetermined
        } else {
            Depth::TopLevel
        };

        Self {
            uuid: *uuid,
            fun: called_fun,
            params: params_vec,
            prefix,
            location: id.location.clone(),
            accessed_as: name.clone(),
            depth,
            name_matches_global_arg: false,
            name_matches_instance_arg: false,
        }
    }

    pub fn finish_setup(&mut self, parent_fun: Option<&Fun>) {
        let Some(parent_fun) = parent_fun else {
            // Function call is top level, no need for further setup
            self.depth = Depth::TopLevel;
            return;
        };
        self.depth = Depth::Nested {
            parent_fun: parent_fun.uuid,
        };

        self.name_matches_instance_arg = parent_fun
            .modifiers
            .get(&ModifierKind::Instance)
            .is_some_and(|modifiers| {
                modifiers.iter().any(|modifier| {
                    modifier.args.iter().any(|arg| {
                        if arg.name.to_lower() == self.accessed_as.to_lower() {
                            // Exact match e.g. `instance(foo.bar)` and `foo.bar()`
                            return true;
                        }
                        let Some(prefix) = &self.prefix else {
                            // Not an exact match and no function call prefix
                            return false;
                        };
                        if prefix.to_ascii_lowercase() == arg.name.to_lower() {
                            // Function call prefix matches instance arg name e.g. `instance(foo)` and `foo.bar()`
                            return true;
                        }
                        // Check if argument is part of the prefix e.g. `instance(foo.bar)` and `foo.bar.baz()`
                        let prefix_dots = Dots::new(prefix);
                        let prefix_lower = prefix.to_ascii_lowercase();
                        for prefix_dot in prefix_dots {
                            let split = &prefix_lower[..prefix_dot];
                            if arg.name.to_lower() == split {
                                return true;
                            }
                        }
                        false
                    })
                })
            });
        self.name_matches_global_arg = !self.name_matches_instance_arg
            && self.prefix.is_some()
            && parent_fun.has_modifier_with_arg_name(&ModifierKind::Global, &self.accessed_as);
    }
}
