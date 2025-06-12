use std::{collections::HashMap, error::Error, rc::Rc};

use crate::functions::{Depth, Fun};
use crate::{
    file::File, first_pass::FirstPass, issue::IssueTracker, lints, meta::Meta, scopes::GlobalScope,
    section::Section, symbols,
};

pub struct Program {
    pub metas: Vec<Meta>,
    pub sections: HashMap<&'static str, Section>,
    pub scope: GlobalScope,
}

impl Program {
    pub fn lint(&self, issues: &mut IssueTracker) {
        for lint in lints::get() {
            lint(self, issues);
        }
    }
    pub fn from_file(
        entry_file: &Rc<File>,
        issues: &mut IssueTracker,
    ) -> Result<Self, Box<dyn Error>> {
        // First pass is EEL to AST and metas parsing.
        let mut first_pass = FirstPass::from_file_recursive(entry_file, issues)?;
        // Second pass is resolving symbols (create scopes, collect functions, ...)
        let scope = symbols::collect(&mut first_pass);
        Ok(Self {
            metas: first_pass.metas,
            sections: first_pass.sections,
            scope,
        })
    }
    pub fn has_top_level_calls(&self, fun_def: &Rc<Fun>) -> bool {
        self.sections.values().any(|section| {
            section.fun_calls.iter().any(|call| {
                matches!(call.depth, Depth::TopLevel)
                    && call
                        .fun
                        .as_ref()
                        .is_some_and(|called_fun| called_fun.uuid == fun_def.uuid)
            })
        })
    }
}
