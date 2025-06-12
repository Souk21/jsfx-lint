use crate::ast::Ast;
use crate::functions::{Fun, Param, ParamKind};
use crate::located_ast::LocatedAst;

pub fn collect(params_opt: &Option<Vec<LocatedAst>>, fun: &Fun) -> Vec<Vec<Param>> {
    let mut params_vec = Vec::new();
    let Some(params) = params_opt else {
        return params_vec;
    };
    for param in params {
        let mut this_param_vec = Vec::new();
        let rets = param.get_return_values();
        for ret in rets {
            match &ret.ast {
                Ast::Identifier { value, .. } | Ast::StringIdentifier { value, .. } => {
                    let kind = ParamKind::Identifier {
                        name: value.clone(),
                    };
                    this_param_vec.push(Param {
                        kind,
                        location: ret.location.clone(),
                    });
                }
                Ast::Number(_)
                | Ast::Add { .. }
                | Ast::CharLit { .. }
                | Ast::Cmp { .. }
                | Ast::Unary { .. }
                | Ast::Sub { .. }
                | Ast::Pow { .. }
                | Ast::Mul { .. }
                | Ast::Div { .. }
                | Ast::ModShift { .. }
                | Ast::MemoryAccess { .. }
                | Ast::Loop { .. }
                | Ast::FunCall { .. }
                | Ast::While { .. }
                | Ast::LogicalAndOr { .. }
                | Ast::Assignment { .. }
                | Ast::AndOr { .. } => this_param_vec.push(Param {
                    kind: ParamKind::OtherValue,
                    location: ret.location.clone(),
                }),
                Ast::String { .. } => {
                    let str_value = ret.ast.get_entire_string();
                    let kind = ParamKind::StringValue { value: str_value };
                    this_param_vec.push(Param {
                        kind,
                        location: ret.location.clone(),
                    });
                }
                Ast::Fun { .. }
                | Ast::Arg { .. }
                | Ast::Program { .. }
                | Ast::Unnecessary { .. }
                | Ast::If { .. }
                | Ast::Void
                | Ast::Compound { .. }
                | Ast::FunMod { .. } => {
                    panic!("Unexpected AST in {}() returns {:#?}", fun.name, ret);
                }
            }
        }
        params_vec.push(this_param_vec);
    }
    params_vec
}
