use crate::ast::Ast;
use crate::functions::Arg;
use crate::located_ast::LocatedAst;

pub(super) fn collect_args(args_ast: &Vec<LocatedAst>) -> Vec<Arg> {
    let mut args_vec = Vec::new();
    for a in args_ast {
        let Ast::Arg { identifier, .. } = &a.ast else {
            panic!("Expected Ast::Arg")
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
            panic!("Identifier should be Ast::Identifier or Ast::StringIdentifier");
        };
        let is_str = matches!(identifier.ast, Ast::StringIdentifier { .. });
        args_vec.push(Arg {
            name: arg_name.clone(),
            is_ref: *is_ref,
            is_str,
            optional: false,
            location: Some(identifier.location.clone()),
        });
    }
    args_vec
}
