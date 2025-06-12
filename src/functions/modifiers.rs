use crate::ast::Ast;
use crate::functions::{Arg, Modifier, ModifierKind};
use crate::located_ast::LocatedAst;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

impl Display for ModifierKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global => write!(f, "global"),
            Self::Local => write!(f, "local"),
            Self::Instance => write!(f, "instance"),
        }
    }
}

pub(super) fn collect_fn_mods(
    modifiers_opt: &Option<Vec<LocatedAst>>,
) -> HashMap<ModifierKind, Vec<Modifier>> {
    let mut hashmap = HashMap::new();
    let Some(modifiers) = modifiers_opt else {
        // No modifiers
        return hashmap;
    };
    for modifier in modifiers {
        let Ast::FunMod { kind, args, .. } = &modifier.ast else {
            panic!("Expected FnMod");
        };

        let mut modifier = Modifier {
            args: Vec::new(),
            location: modifier.location.clone(),
        };

        if let Some(args) = args {
            for arg in args {
                let Ast::Arg { identifier, .. } = &arg.ast else {
                    panic!("Unexpected modifier argument");
                };
                let (Ast::Identifier {
                    value: arg_name,
                    is_ref,
                }
                | Ast::StringIdentifier {
                    value: arg_name,
                    is_ref,
                }) = &identifier.ast
                else {
                    panic!("Unexpected identifier");
                };

                let is_str = matches!(identifier.ast, Ast::StringIdentifier { .. });
                modifier.args.push(Arg {
                    name: arg_name.clone(),
                    is_ref: *is_ref,
                    is_str,
                    optional: false,
                    location: Some(identifier.location.clone()),
                });
            }
        }
        hashmap.entry(kind.clone()).or_default().push(modifier);
    }
    hashmap
}
