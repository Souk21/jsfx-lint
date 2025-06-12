use crate::access;
use crate::scopes::FunScope;

impl FunScope {
    pub const fn new() -> Self {
        Self {
            accesses: Vec::new(),
            returns: Vec::new(),
        }
    }
    pub fn add_access(&mut self, access: access::WithinFunction) {
        self.accesses.push(access);
    }
}

impl Default for FunScope {
    fn default() -> Self {
        Self::new()
    }
}
