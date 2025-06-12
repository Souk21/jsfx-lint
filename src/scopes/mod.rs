mod fun_scope;
mod global_scope;

use crate::access;
use crate::access::Return;
use crate::variables::{BuiltinVar, Variable};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct GlobalScope {
    pub variables: HashMap<String, Variable>,
    pub builtin_vars: HashMap<String, Rc<BuiltinVar>>,
}

#[derive(Debug)]
pub struct FunScope {
    pub accesses: Vec<access::WithinFunction>,
    /// Returns of the function. All `Return` here are `ReturnKind::Value`
    pub returns: Vec<Return>,
}
