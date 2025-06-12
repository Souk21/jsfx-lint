%token IDENTIFIER TOKEN_SHL TOKEN_SHR
%token TOKEN_LTE TOKEN_GTE TOKEN_EQ TOKEN_EQ_EXACT TOKEN_NE TOKEN_NE_EXACT TOKEN_LOGICAL_AND TOKEN_LOGICAL_OR
%token TOKEN_ADD_OP TOKEN_SUB_OP TOKEN_MOD_OP TOKEN_OR_OP TOKEN_AND_OP TOKEN_XOR_OP TOKEN_DIV_OP TOKEN_MUL_OP TOKEN_POW_OP
%token STRING_LITERAL STRING_IDENTIFIER
%expect 75
%parse-param parser_param: &crate::parser::ParserParam
%start program

%%
unmatched -> ():
    "UNMATCHED" {  }
    ;
string_id -> Result<LocatedAst, ParseError>:
    STRING_IDENTIFIER {
        Ok(LocatedAst{
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
            ast: Ast::StringIdentifier{
                value:span(&$1?.span(), parser_param),
                is_ref: false
            },
        })
    }
    ;

char_literal -> Result<LocatedAst, ParseError>:
    "CHAR_LITERAL" {
        let str = span(&$1?.span(), parser_param);
        let str = str.substr(1 .. str.len() - 1);
        Ok(LocatedAst{
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
            ast: Ast::CharLit(str),
        })
    }
    ;

VALUE -> Result<LocatedAst, ParseError>:
    "NUMBER" { Ok(LocatedAst{ast: Ast::Number(span(&$1?.span(), parser_param)), location: line_col_to_location($lexer.line_col($1?.span()), parser_param)}) }
    | "HEX" { Ok(LocatedAst{ast: Ast::Number(span(&$1?.span(), parser_param)), location: line_col_to_location($lexer.line_col($1?.span()), parser_param)}) }
    | "MASK" { Ok(LocatedAst{ast: Ast::Number(span(&$1?.span(), parser_param)), location: line_col_to_location($lexer.line_col($1?.span()), parser_param)}) }
    | "CHAR" { Ok(LocatedAst{ast: Ast::Number(span(&$1?.span(), parser_param)), location: line_col_to_location($lexer.line_col($1?.span()), parser_param)}) }
    ;
More_params -> Result<Vec<LocatedAst>, ParseError>:
	expression { Ok(vec![$1?]) }
	| More_params ',' expression {
        let mut more_params = $1?;
        more_params.push($3?);
        Ok(more_params)
    }
	;

string -> Result<LocatedAst, ParseError>:
    STRING_LITERAL {
        let str = span(&$1?.span(), parser_param);
        let str = str.substr(1 .. str.len() - 1);
        Ok(LocatedAst {
            ast: Ast::String {
                value: str,
                next: None,
            },
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
        })
    }
    | STRING_LITERAL string {
        let str = span(&$1?.span(), parser_param);
        let str = str.substr(1 .. str.len() - 1);
        Ok(LocatedAst {
            ast: Ast::String {
                value: str,
                next: Some(Box::new($2?)),
            },
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
        })
    }
    ;

assignable_value -> Result<LocatedAst, ParseError>:
    id { $1 }
    | '(' expression ')' { $2 }
    | id '(' expression ')' '(' expression ')' {
        if id_match(&$1, "while") {
            Ok(LocatedAst {
                ast: Ast::While {
                    condition: Some(Box::new($3?)),
                    body: Some(Box::new($6?)),
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($7?.span()), parser_param),
                    parser_param,
                ),
            })
        } else {
            Err(ParseError::ExpectedWhile($1?.location))
        }
    }
    | id '(' ')' {
        if id_match(&$1, "while") {
            Ok(LocatedAst {
                ast: Ast::While {
                    condition: None,
                    body: None,
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($3?.span()), parser_param),
                    parser_param,
                ),
            })
        } else {
            let one = $1?;
            return Ok(LocatedAst {
                location: from_to(
                    &one.location,
                    &line_col_to_location($lexer.line_col($3?.span()), parser_param),
                    parser_param,
                ),
                ast: Ast::FunCall {
                    name: Box::new(one),
                    params: None,
                    uuid: next_id(),
                },
            });
        }
    }
    | id '(' expression ')' {
        if id_match(&$1, "while") {
            Ok(LocatedAst {
                ast: Ast::While {
                    condition: Some(Box::new($3?)),
                    body: None,
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                    parser_param,
                ),
            })
        } else {
            let one = $1?;
            Ok(LocatedAst {
                location: from_to(
                    &one.location,
                    &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                    parser_param
                ),
                ast: Ast::FunCall {
                    name: Box::new(one),
                    params: Some(vec![$3?]),
                    uuid: next_id(),
                },
            })
        }
    }
    | id '(' expression ',' expression ')' {
        if id_match(&$1, "loop") {
            // Can be loop only if 2 params
            Ok(LocatedAst {
                ast: Ast::Loop {
                    count: Box::new($3?),
                    body: Box::new($5?),
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($6?.span()), parser_param),
                    parser_param,
                ),
            })
        } else {
            let one = $1?;
            return Ok(LocatedAst {
                location: from_to(
                    &one.location,
                    &line_col_to_location($lexer.line_col($6?.span()), parser_param),
                    parser_param,
                ),
                ast: Ast::FunCall {
                    name: Box::new(one),
                    params: Some(vec![$3?, $5?]),
                    uuid: next_id(),
                },
            });
        }
    }
    | id '(' expression ',' expression ',' More_params ')'  {
        let mut more_params = $7?;
        more_params.insert(0, $5?);
        more_params.insert(0, $3?);
        let one = $1?;
        return Ok(LocatedAst {
            location: from_to(
                &one.location,
                &line_col_to_location($lexer.line_col($8?.span()), parser_param),
                parser_param,
            ),
            ast: Ast::FunCall {
                name: Box::new(one),
                params: Some(more_params),
                uuid: next_id(),
            },
        });
    }
    | rvalue '[' ']' {
        let one = $1?;
        return Ok(LocatedAst {
            location: from_to(
                &one.location,
                &line_col_to_location($lexer.line_col($3?.span()), parser_param),
                parser_param,
            ),
            ast: Ast::MemoryAccess {
                rvalue: Box::new(one),
                index: None,
            },
        });
    }
    | rvalue '[' expression ']' {
        let one = $1?;
        return Ok(LocatedAst {
            location: from_to(
                &one.location,
                &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                parser_param,
            ),
            ast: Ast::MemoryAccess {
                rvalue: Box::new(one),
                index: Some(Box::new($3?)),
            },
        });
    }
    ;

rvalue -> Result<LocatedAst, ParseError>:
	VALUE { $1 }
    | string_id { $1 }
    | string { $1 }
    | char_literal { $1 }
    | assignable_value { $1 }
    ;


assignment -> Result<LocatedAst, ParseError>:
    rvalue { $1 }
    | assignable_value '=' if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Assign,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_ADD_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Add,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_SUB_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Sub,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_MOD_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Mod,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_OR_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Or,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_AND_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::And,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_XOR_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Xor,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_DIV_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Div,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_MUL_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Mul,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | assignable_value TOKEN_POW_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Pow,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | string_id '=' if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Assign,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | string_id TOKEN_ADD_OP if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Assignment {
                operator: AssignmentOperator::Add,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    ;

unary_expr -> Result<LocatedAst, ParseError>:
    assignment { $1 }
	| '+' unary_expr {
        let two = $2?;
        return Ok(LocatedAst {
            location: from_to(
                &line_col_to_location($lexer.line_col($1?.span()), &parser_param),
                &two.location,
                parser_param,
            ),
            ast: Ast::Unary {
                operator: UnaryOperator::Pos,
                operand: Box::new(two),
            },
        });
    }
	| '-' unary_expr {
        let two = $2?;
        return Ok(LocatedAst {
            location: from_to(
                &line_col_to_location($lexer.line_col($1?.span()), &parser_param),
                &two.location,
                parser_param,
            ),
            ast: Ast::Unary {
                operator: UnaryOperator::Neg,
                operand: Box::new(two),
            },
        });
    }
	| '!' unary_expr {
        let two = $2?;
        return Ok(LocatedAst {
            location: from_to(
                &line_col_to_location($lexer.line_col($1?.span()), &parser_param),
                &two.location,
                parser_param,
            ),
            ast: Ast::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(two),
            },
        });
    }
	;

pow_expr -> Result<LocatedAst, ParseError>:
    unary_expr { $1 }
    | pow_expr '^' unary_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Pow {
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    ;

mod_expr -> Result<LocatedAst, ParseError>:
    pow_expr { $1 }
    | mod_expr '%' pow_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::ModShift {
                operator: ModShiftOperator::Mod,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | mod_expr TOKEN_SHL pow_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::ModShift {
                operator: ModShiftOperator::Left,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | mod_expr TOKEN_SHR pow_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::ModShift {
                operator: ModShiftOperator::Right,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    ;

div_expr -> Result<LocatedAst, ParseError>:
	mod_expr { $1 }
	| div_expr '/' mod_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Div {
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	;


mul_expr -> Result<LocatedAst, ParseError>:
	div_expr { $1 }
	| mul_expr '*' div_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Mul {
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	;


sub_expr -> Result<LocatedAst, ParseError>:
	mul_expr { $1 }
	| sub_expr '-' mul_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Sub {
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	;

add_expr -> Result<LocatedAst, ParseError>:
	sub_expr { $1 }
	| add_expr '+' sub_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Add {
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	;

andor_expr -> Result<LocatedAst, ParseError>:
	add_expr { $1 }
	| andor_expr '&' add_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::AndOr {
                operator: AndOrOperator::And,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	| andor_expr '|' add_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::AndOr {
                operator: AndOrOperator::Or,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	| andor_expr '~' add_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::AndOr {
                operator: AndOrOperator::Xor,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
	;

cmp_expr -> Result<LocatedAst, ParseError>:
    andor_expr { $1 }
    | cmp_expr '<' andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::Lt,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr '>' andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::Gt,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr TOKEN_LTE andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::Lte,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr TOKEN_GTE andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::Gte,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr TOKEN_EQ andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::Eq,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr TOKEN_EQ_EXACT andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::ExactEq,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr TOKEN_NE andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::Ne,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | cmp_expr TOKEN_NE_EXACT andor_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::Cmp {
                operator: CmpOperator::ExactNe,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    ;

logical_and_or_expr -> Result<LocatedAst, ParseError>:
    cmp_expr { $1 }
    | logical_and_or_expr TOKEN_LOGICAL_AND cmp_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::LogicalAndOr {
                operator: LogicalAndOrOperator::And,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    | logical_and_or_expr TOKEN_LOGICAL_OR cmp_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::LogicalAndOr {
                operator: LogicalAndOrOperator::Or,
                lhs: Box::new(one),
                rhs: Box::new(three),
            },
        });
    }
    ;

if_else_expr -> Result<LocatedAst, ParseError>:
    logical_and_or_expr { $1 }
    | logical_and_or_expr '?' if_else_expr ':' if_else_expr {
        let condition = $1?;
        let no = $5?;
        return Ok(LocatedAst {
            location: from_to(&condition.location, &no.location, parser_param),
            ast: Ast::If {
                condition: Box::new(condition),
                yes: Some(Box::new($3?)),
                no: Some(Box::new(no)),
            },
        });
    }
    | logical_and_or_expr '?' ':' if_else_expr {
        let one = $1?;
        let four = $4?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &four.location, parser_param),
            ast: Ast::If {
                condition: Box::new(one),
                yes: None,
                no: Some(Box::new(four)),
            },
        });
    }
    | logical_and_or_expr '?' if_else_expr {
        let one = $1?;
        let three = $3?;
        return Ok(LocatedAst {
            location: from_to(&one.location, &three.location, parser_param),
            ast: Ast::If {
                condition: Box::new(one),
                yes: Some(Box::new(three)),
                no: None,
            },
        });
    }
    ;

expression -> Result<LocatedAst, ParseError>:
	if_else_expr {
	    let inner = $1?;
	    Ok(LocatedAst{
	        location: inner.location.clone(),
	        ast: Ast::Compound{expressions:vec![inner], extra_semicolon: None},
        })
    }
	| expression ";" if_else_expr {
        let right = $3?;
        let mut left = $1?;
        left.location = from_to(&left.location, &right.location.clone(), parser_param);
        if let Ast::Compound{ref mut expressions, ..} = left.ast {
            expressions.push(right);
        }
        return Ok(left);
    }
	| expression ";" {
        let inner = $1?;
        return Ok(LocatedAst {
            location: from_to(
                &inner.location.clone(),
                &line_col_to_location($lexer.line_col($2?.span()), parser_param),
                parser_param,
            ),
            ast: Ast::Compound{expressions:vec![inner], extra_semicolon: None},
        });
    }
	;

function_mod -> Result<LocatedAst, ParseError>:
    id "(" args_ref_or_comma_or_string_id ")" {
        if id_match(&$1, "local") || id_match(&$1, "static") {
            match $3? {
                (_, Some(c)) => {
                    return Ok(LocatedAst {
                        ast: Ast::FunMod {
                            kind: ModifierKind::Local,
                            args: None,
                            commas: Some(Box::new(c)),
                        },
                        location: from_to(
                            &$1?.location,
                            &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                            parser_param,
                        ),
                    });
                }
                (a, _) => {
                    return Ok(LocatedAst {
                        ast: Ast::FunMod {
                            kind: ModifierKind::Local,
                            args: a,
                            commas: None,
                        },
                        location: from_to(
                            &$1?.location,
                            &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                            parser_param,
                        ),
                    });
                }
            }
        } else if id_match(&$1, "global") || id_match(&$1, "globals") {
            match $3? {
                (_, Some(c)) => {
                    return Ok(LocatedAst {
                        ast: Ast::FunMod {
                            kind: ModifierKind::Global,
                            args: None,
                            commas: Some(Box::new(c)),
                        },
                        location: from_to(
                            &$1?.location,
                            &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                            parser_param,
                        ),
                    });
                }
                (a, _) => {
                    return Ok(LocatedAst {
                        ast: Ast::FunMod {
                            kind: ModifierKind::Global,
                            args: a,
                            commas: None,
                        },
                        location: from_to(
                            &$1?.location,
                            &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                            parser_param,
                        ),
                    });
                }
            }
        } else if id_match(&$1, "instance") {
            match $3? {
                (_, Some(c)) => {
                    return Ok(LocatedAst {
                        ast: Ast::FunMod {
                            kind: ModifierKind::Instance,
                            args: None,
                            commas: Some(Box::new(c)),
                        },
                        location: from_to(
                            &$1?.location,
                            &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                            parser_param,
                        ),
                    });
                }
                (a, _) => {
                    return Ok(LocatedAst {
                        ast: Ast::FunMod {
                            kind: ModifierKind::Instance,
                            args: a,
                            commas: None,
                        },
                        location: from_to(
                            &$1?.location,
                            &line_col_to_location($lexer.line_col($4?.span()), parser_param),
                            parser_param,
                        ),
                    });
                }
            }
        }
        Err(ParseError::ExpectedFunModifier($1?.location))
    }
    | id "(" ")" {
        if id_match(&$1, "instance") {
            return Ok(LocatedAst {
                ast: Ast::FunMod {
                    kind: ModifierKind::Instance,
                    args: None,
                    commas: None,
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($3?.span()), parser_param),
                    parser_param,
                ),
            });
        } else if id_match(&$1, "local") || id_match(&$1, "static") {
            return Ok(LocatedAst {
                ast: Ast::FunMod {
                    kind: ModifierKind::Local,
                    args: None,
                    commas: None,
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($3?.span()), parser_param),
                    parser_param,
                ),
            });
        } else if id_match(&$1, "global") || id_match(&$1, "globals") {
            return Ok(LocatedAst {
                ast: Ast::FunMod {
                    kind: ModifierKind::Global,
                    args: None,
                    commas: None,
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($3?.span()), parser_param),
                    parser_param,
                ),
            });
        }
        Err(ParseError::ExpectedFunModifier($1?.location))
    }
    ;

function_mods -> Result<Vec<LocatedAst>, ParseError>:
    function_mod { Ok(vec![$1?]) }
    | function_mods function_mod {
        let mut first = $1?;
        first.push($2?);
        Ok(first)
    }
    ;
id -> Result<LocatedAst, ParseError>:
    IDENTIFIER {
        Ok(LocatedAst {
            ast: Ast::Identifier {
                value: span(&$1?.span(), parser_param),
                is_ref: false,
            },
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
        })
    }
    ;

id_ref -> Result<LocatedAst, ParseError>:
    IDENTIFIER {
        Ok(LocatedAst {
            ast: Ast::Identifier {
                value: span(&$1?.span(), parser_param),
                is_ref: false,
            },
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
        })
    }
    | IDENTIFIER "*" {
        Ok(LocatedAst {
            ast: Ast::Identifier {
                value: span(&$1?.span(), parser_param),
                is_ref: true,
            },
            location: from_to(
                &line_col_to_location($lexer.line_col($1?.span()), &parser_param),
                &line_col_to_location($lexer.line_col($2?.span()), parser_param),
                parser_param,
            ),
        })
    }
    ;

string_id_ref -> Result<LocatedAst, ParseError>:
    STRING_IDENTIFIER {
        Ok(LocatedAst {
            ast: Ast::StringIdentifier {
                value: span(&$1?.span(), parser_param),
                is_ref: false,
            },
            location: line_col_to_location($lexer.line_col($1?.span()), parser_param),
        })
    }
    | STRING_IDENTIFIER "*" {
        Ok(LocatedAst {
            ast: Ast::StringIdentifier {
                value: span(&$1?.span(), parser_param),
                is_ref: true,
            },
            location: from_to(
                &line_col_to_location($lexer.line_col($1?.span()), &parser_param),
                &line_col_to_location($lexer.line_col($2?.span()), parser_param),
                parser_param,
            ),
        })
    }
    ;

args_ref_or_comma_or_string_id -> Result<(Option<Vec<LocatedAst>>, Option<LocatedAst>), ParseError>:
    args_ref_or_string_id { Ok((Some($1?), None)) }
    | args_ref_or_string_id MANY_COMMA {
        let mut args = $1?;
        if let Some(last) = args.last_mut() {
            let ast = &mut last.ast;
            if let Ast::Arg {
                trailing_comma,
                ..
            } = ast
            {
                *trailing_comma = Some($2?);
            }
        }
        Ok((Some(args), None))
    }
    | MANY_COMMA {
        let inner = $1?;
        return Ok((
            None,
            Some(LocatedAst {
                ast: Ast::Unnecessary {
                    lex: ",".repeat(inner.0),
                },
                location: inner.1,
            }),
        ));
    }
    ;

id_ref_or_string_id -> Result<LocatedAst, ParseError>:
    id_ref { $1 }
    | string_id_ref { $1 }
    ;

args_ref_or_string_id -> Result<Vec<LocatedAst>, ParseError>:
    id_ref_or_string_id {
        let inner = $1?;
        Ok(vec![LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Arg {
                identifier: Box::new(inner),
                leading_commas: None,
                trailing_comma: None,
            },
        }])
    }
    | MANY_COMMA id_ref_or_string_id {
        let inner = $2?;
        Ok(vec![LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Arg {
                identifier: Box::new(inner),
                leading_commas: Some($1?),
                trailing_comma: None,
            },
        }])
    }
    | args_ref_or_string_id MANY_COMMA id_ref_or_string_id {
        let mut args = $1?;
        let inner = $3?;
        let loc_first_arg = &args[0].location;
        args.push(LocatedAst {
            location: from_to(loc_first_arg, &inner.location, parser_param),
            ast: Ast::Arg {
                identifier: Box::new(inner),
                leading_commas: Some($2?),
                trailing_comma: None,
            },
        });
        Ok(args)
    }
    | args_ref_or_string_id id_ref_or_string_id {
        let mut args = $1?;
        let right = $2?;
        let loc_first_arg = &args[0].location;
        args.push(LocatedAst {
            location: from_to(loc_first_arg, &right.location, parser_param),
            ast: Ast::Arg {
                identifier: Box::new(right),
                leading_commas: None,
                trailing_comma: None,
            },
        });
        Ok(args)
    }
    ;

MANY_COMMA -> Result<(usize, Location), ParseError>:
    "," {
        Ok((
            1,
            line_col_to_location($lexer.line_col($1?.span()), parser_param),
        ))
    }
    | MANY_COMMA "," {
        let commas = $1?;
        return Ok((
            commas.0 + 1,
            from_to(
                &commas.1,
                &line_col_to_location($lexer.line_col($2?.span()), parser_param),
                parser_param,
            ),
        ));
    }
    ;

args_ref -> Result<Vec<LocatedAst>, ParseError>:
    id_ref {
        let id_ref = $1?;
        return Ok(vec![LocatedAst {
            location: id_ref.location.clone(),
            ast: Ast::Arg {
                identifier: Box::new(id_ref),
                leading_commas: None,
                trailing_comma: None,
            },
        }]);
    }
    | MANY_COMMA id_ref {
        let id_ref = $2?;
        return Ok(vec![LocatedAst {
            location: id_ref.location.clone(),
            ast: Ast::Arg {
                identifier: Box::new(id_ref),
                leading_commas: Some($1?),
                trailing_comma: None,
            },
        }]);
    }
    | args_ref MANY_COMMA id_ref {
        let mut args = $1?;
        let id_ref = $3?;
        let location_1 = &args[0].location;
        args.push(LocatedAst {
            location: from_to(location_1, &id_ref.location, parser_param),
            ast: Ast::Arg {
                identifier: Box::new(id_ref),
                leading_commas: Some($2?),
                trailing_comma: None,
            },
        });
        Ok(args)
    }
    | args_ref id_ref {
        let mut args = $1?;
        let id_ref = $2?;
        let location_1 = &args[0].location;
        args.push(LocatedAst {
            location: from_to(location_1, &id_ref.location, parser_param),
            ast: Ast::Arg {
                identifier: Box::new(id_ref),
                leading_commas: None,
                trailing_comma: None,
            },
        });
        Ok(args)
    }
    ;

ref_args_or_comma -> Result<(Option<Vec<LocatedAst>>, Option<LocatedAst>), ParseError>:
    args_ref { Ok((Some($1?), None)) }
    | args_ref MANY_COMMA {
        let mut args = $1?;
        if let Some(ref mut last) = args.last_mut() {
            let ast = &mut last.ast;
            if let Ast::Arg {
                trailing_comma,
                ..
            } = ast
            {
                *trailing_comma = Some($2?);
            }
        }
        Ok((Some(args), None))
    }
    | MANY_COMMA {
        let inner = $1?;
        return Ok((
            None,
            Some(LocatedAst {
                location: inner.1,
                ast: Ast::Unnecessary {
                    lex: ",".repeat(inner.0),
                },
            }),
        ));
    }
    ;

function -> Result<LocatedAst, ParseError>:
    id id "(" ref_args_or_comma ")" function_mods "(" expression ")" {
        if !id_match(&$1, "function") {
            return Err(ParseError::ExpectedFunction($1?.location))
        }
        match $4? {
            (_, Some(c)) => Ok(LocatedAst {
                ast: Ast::Fun {
                    identifier: Box::new($2?),
                    args: None,
                    modifiers: Some($6?),
                    commas: Some(Box::new(c)),
                    body: Box::new($8?),
                    parens: true,
                    uuid: next_id(),
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($9?.span()), parser_param),
                    parser_param,
                ),
            }),
            (a, _) => Ok(LocatedAst {
                ast: Ast::Fun {
                    identifier: Box::new($2?),
                    args: a,
                    modifiers: Some($6?),
                    commas: None,
                    body: Box::new($8?),
                    parens: true,
                    uuid: next_id(),
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($9?.span()), parser_param),
                    parser_param,
                ),
            }),
        }
    }
    | id id "(" ")" function_mods "(" expression ")" {
        if !id_match(&$1, "function") {
            return Err(ParseError::ExpectedFunction($1?.location))
        }
        Ok(LocatedAst {
            ast: Ast::Fun {
                identifier: Box::new($2?),
                args: None,
                modifiers: Some($5?),
                body: Box::new($7?),
                commas: None,
                parens: true,
                uuid: next_id(),
            },
            location: from_to(
                &$1?.location,
                &line_col_to_location($lexer.line_col($8?.span()), parser_param),
                parser_param,
            ),
        })
    }
    | id id "(" ref_args_or_comma ")" "(" expression ")" {
        if !id_match(&$1, "function") {
            return Err(ParseError::ExpectedFunction($1?.location))
        }
        match $4? {
            (_, Some(c)) => Ok(LocatedAst {
                ast: Ast::Fun {
                    identifier: Box::new($2?),
                    args: None,
                    modifiers: None,
                    commas: Some(Box::new(c)),
                    body: Box::new($7?),
                    parens: true,
                    uuid: next_id(),
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($8?.span()), parser_param),
                    parser_param,
                ),
            }),
            (a, _) => Ok(LocatedAst {
                ast: Ast::Fun {
                    identifier: Box::new($2?),
                    args: a,
                    modifiers: None,
                    commas: None,
                    body: Box::new($7?),
                    parens: true,
                    uuid: next_id(),
                },
                location: from_to(
                    &$1?.location,
                    &line_col_to_location($lexer.line_col($8?.span()), parser_param),
                    parser_param,
                ),
            }),
        }
    }
    | id id "(" ")" "(" expression ")" {
        if !id_match(&$1, "function") {
            return Err(ParseError::ExpectedFunction($1?.location))
        }
        Ok(LocatedAst {
            ast: Ast::Fun {
                identifier: Box::new($2?),
                args: None,
                modifiers: None,
                body: Box::new($6?),
                commas: None,
                parens: true,
                uuid: next_id(),
            },
            location: from_to(
                &$1?.location,
                &line_col_to_location($lexer.line_col($7?.span()), parser_param),
                parser_param,
            ),
        })
    }
    | id id function_mods "(" expression ")" {
        if !id_match(&$1, "function") {
            return Err(ParseError::ExpectedFunction($1?.location))
        }
        Ok(LocatedAst {
            ast: Ast::Fun {
                identifier: Box::new($2?),
                args: None,
                modifiers: Some($3?),
                body: Box::new($5?),
                commas: None,
                parens: false,
                uuid: next_id(),
            },
            location: from_to(
                &$1?.location,
                &line_col_to_location($lexer.line_col($6?.span()), parser_param),
                parser_param,
            ),
        })
    }
    ;

MANY_SEMICOLON -> Result<(usize, Location), ParseError>:
    ";" { Ok((
            1,
            line_col_to_location($lexer.line_col($1?.span()), parser_param),
        ))
    }
    | MANY_SEMICOLON ";" { 
        let commas = $1?;
        return Ok((
            commas.0 + 1,
            from_to(
                &commas.1,
                &line_col_to_location($lexer.line_col($2?.span()), parser_param),
                parser_param,
            ),
        ));
    }
    ;

expr_or_func -> Result<LocatedAst, ParseError>:
    if_else_expr {
        let inner = $1?;
        Ok(LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Compound{expressions:vec![inner], extra_semicolon: None},
        })
    }
    | function {
        let inner = $1?;
        Ok(LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Compound{expressions:vec![inner], extra_semicolon: None},
        })
    }
    | if_else_expr MANY_SEMICOLON expr_or_func {
        let mut inner = $3?;
        let fun = $1?;
        inner.location = from_to(&fun.location, &inner.location, parser_param);
        if let Ast::Compound{ref mut expressions, ref mut extra_semicolon} = inner.ast {
            expressions.insert(0, fun);
            *extra_semicolon = Some($2?);
        }
        Ok(inner)
    }
    | function MANY_SEMICOLON expr_or_func {
        let mut inner = $3?;
        let fun = $1?;
        inner.location = from_to(&fun.location, &inner.location, parser_param);
        if let Ast::Compound{ref mut expressions, ref mut extra_semicolon} = inner.ast {
            expressions.insert(0, fun);
            *extra_semicolon = Some($2?);
        }
        Ok(inner)
    }
    | if_else_expr MANY_SEMICOLON {
        let inner = $1?;
        return Ok(LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Compound{expressions:vec![inner], extra_semicolon: Some($2?)},
        });
    }
    | function MANY_SEMICOLON {
        let inner = $1?;
        return Ok(LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Compound{expressions:vec![inner], extra_semicolon: Some($2?)},
        });
    }
    ;

program -> Result<LocatedAst, ParseError>:
    {
        Ok(LocatedAst {
            ast: Ast::Void,
            location: empty_line_col(parser_param),
        })
    }
	| expr_or_func {
        let inner = $1?;
        return Ok(LocatedAst {
            location: inner.location.clone(),
            ast: Ast::Program(Box::new(inner)),
        });
    }
	;


%%

use crate::ast::Ast;
use crate::located_ast::LocatedAst;
use crate::location::Location;
use crate::functions::ModifierKind;
use crate::operators::{
    AndOrOperator, AssignmentOperator, CmpOperator, LogicalAndOrOperator, ModShiftOperator,
    UnaryOperator,
};
use crate::rcsubstring::RcSubString;
use crate::parser::{ParserParam, ParseError};
use crate::location::LineCol;
use lrpar::Span;
use uuid::Uuid;

fn span(s: &Span, parser_param: &ParserParam) -> RcSubString {
    parser_param.source.substr(s.start()..s.end())
}

fn next_id() -> Uuid {
    Uuid::new_v4()
}

fn id_match(
    id: &Result<LocatedAst, ParseError>,
    cmp: &str,
) -> bool {
    match id {
        Ok(inner) => match &inner.ast {
            Ast::Identifier { value, .. } => {
                value.to_ascii_lowercase() == cmp
            }
            _ => false,
        },
        _ => false,
    }
}

fn from_to(from: &Location, to: &Location, parser_param: &ParserParam) -> Location {
    Location {
        file: parser_param.file.clone(),
        line_col: LineCol {
            start_line: from.line_col.start_line,
            end_line: to.line_col.end_line,
            start_column: from.line_col.start_column,
            end_column: to.line_col.end_column,
        },
        section: Some(parser_param.section_kind),
    }
}

fn line_col_to_location(
    line_col: ((usize, usize), (usize, usize)),
    parser_param: &ParserParam,
) -> Location {
    Location {
        file: parser_param.file.clone(),
        line_col: LineCol {
            start_line: parser_param.section_line_pos + line_col.0.0,
            end_line: parser_param.section_line_pos + line_col.1.0,
            start_column: line_col.0.1,
            end_column: line_col.1.1,
        },
        section: Some(parser_param.section_kind),
    }
}

fn empty_line_col(parser_param: &ParserParam) -> Location {
    Location {
        file: parser_param.file.clone(),
        line_col: LineCol{
            start_line: 0,
            end_line: 0,
            start_column: 0,
            end_column: 0,
        },
        section: Some(parser_param.section_kind),
    }
}
