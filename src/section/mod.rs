use std::{collections::HashMap, rc::Rc};

use cfgrammar::TIdx;
use lrpar::Lexeme;
use lrpar::lrpar_mod;
use uuid::Uuid;

use crate::{
    File,
    functions::{self, Fun, FunCall},
    located_ast::LocatedAst,
    rcsubstring::RcSubString,
};
// Brings the parser for `eel2.y` into scope.
lrpar_mod!("../grammar/eel2.y");
mod chunk;

#[derive(Debug)]
pub struct Section {
    pub kind: &'static str,
    pub chunks: Vec<Chunk>,
    pub fun_defs: Vec<Rc<Fun>>,
    pub uuid_to_fun_defs: HashMap<Uuid, Rc<Fun>>,
    pub fun_calls: Vec<Rc<FunCall>>,
    pub uuid_to_fun_calls: HashMap<Uuid, Rc<FunCall>>,
}

#[derive(Debug)]
pub struct Chunk {
    /// 0-based
    pub line_pos: usize,
    pub source: RcSubString,
    pub file: Rc<File>,
    pub params: Option<RcSubString>,
    pub ast: Option<LocatedAst>,
}

impl Section {
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            chunks: Vec::new(),
            fun_defs: Vec::new(),
            uuid_to_fun_defs: HashMap::new(),
            fun_calls: Vec::new(),
            uuid_to_fun_calls: HashMap::new(),
        }
    }

    pub fn find_inexact_obj_function(&self, name: &str) -> Option<Rc<Fun>> {
        let mut longest = None;
        let mut longest_len = 0;
        // Iterating from latest decl to earlier ones
        // So keep the first match, as another match with same len will be a shadowed fn
        for f in self.fun_defs.iter().rev() {
            if f.name.len() <= longest_len {
                continue;
            }
            // name must be long enough to contain function name (and the dot, hence the + 1)
            if name.len() <= f.name.len() + 1 {
                continue;
            }
            // name needs to end with '.' + function name
            if !name
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", &f.name.to_lower()))
            {
                continue;
            }
            longest = Some(f);
            longest_len = f.name.len();
        }
        longest.cloned()
    }

    pub fn find_exact_obj_function(&self, name: &str, param_count: usize) -> Option<Rc<Fun>> {
        let mut longest = None;
        let mut longest_len = 0;
        // Iterating from latest decl to earlier ones
        // So keep the first match, as another match with same len will be a shadowed fn
        for f in self.fun_defs.iter().rev() {
            if f.name.len() <= longest_len {
                continue;
            }
            // name must be long enough to contain function name (and the dot, hence the + 1)
            if name.len() <= f.name.len() + 1 {
                continue;
            }
            // name needs to end with '.' + function name
            if !name
                .to_ascii_lowercase()
                .ends_with(&format!(".{}", &f.name.to_lower()))
            {
                continue;
            }
            // verify argument/params count
            if functions::match_arg_count(param_count, &f.args) {
                longest = Some(f);
                longest_len = f.name.len();
            }
        }
        longest.cloned()
    }

    pub fn find_exact_function(&self, name: &str, param_count: usize) -> Option<Rc<Fun>> {
        self.fun_defs
            .iter()
            .rev()
            .find(|f| {
                let arg_matches = functions::match_arg_count(param_count, &f.args);
                f.name.to_lower() == name.to_ascii_lowercase() && arg_matches
            })
            .cloned()
    }

    pub fn find_inexact_function(&self, name: &str) -> Option<Rc<Fun>> {
        self.fun_defs
            .iter()
            .rev()
            .find(|fun| fun.name.to_lower() == name.to_ascii_lowercase())
            .cloned()
    }

    pub fn print_ast(&self, indent: usize) {
        let indent = " ".repeat(indent);
        println!("{indent}Section @{}", self.kind);
        for chunk in &self.chunks {
            chunk.print_ast(indent.len() + 3);
        }
    }
}

fn get_repair_str(er: &lrpar::ParseError<lrlex::DefaultLexeme, u32>) -> String {
    let mut repair_str = String::new();
    let repair_sequences = er.repairs();
    let repair_sequence = repair_sequences.first();
    if let Some(repair_seq) = repair_sequence {
        repair_str = String::from("Try to");
        for (i, repair) in repair_seq.iter().enumerate() {
            let comma = if i == repair_seq.len() - 1 { "" } else { "," };
            match repair {
                lrpar::ParseRepair::Insert(token) => {
                    repair_str = format!(
                        "{} insert {:?}{comma}",
                        repair_str,
                        eel2_y::token_epp(*token).unwrap_or("")
                    );
                }
                lrpar::ParseRepair::Delete(lexeme) => {
                    repair_str = format!(
                        "{} delete {:?}{comma}",
                        repair_str,
                        eel2_y::token_epp(TIdx(lexeme.tok_id())).unwrap_or("")
                    );
                }
                lrpar::ParseRepair::Shift(lexeme) => {
                    repair_str = format!(
                        "{} shift {:?}{comma}",
                        repair_str,
                        eel2_y::token_epp(TIdx(lexeme.tok_id())).unwrap_or("")
                    );
                }
            }
        }
        repair_str = format!("({repair_str})");
    }
    repair_str
}
