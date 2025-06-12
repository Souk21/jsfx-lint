use crate::access;
use crate::meta::Meta;
use crate::scopes::GlobalScope;
use crate::variables::{BuiltinVar, IsBuiltin, MaybeBoundToSlider, Variable, is_bound_to_slider};
use std::collections::HashMap;
use std::rc::Rc;

impl GlobalScope {
    pub fn new(builtin_vars: HashMap<String, Rc<BuiltinVar>>) -> Self {
        Self {
            variables: HashMap::new(),
            builtin_vars,
        }
    }

    pub fn is_builtin<'a, 'b>(&'b self, name: &str, metas: &'a [Meta]) -> IsBuiltin<'a, 'b> {
        // `_global.*` variables
        // "Like regXX, _global.* are variables shared between all instances of all effects."
        if name.to_ascii_lowercase().starts_with("_global.") {
            return IsBuiltin::Global;
        }
        match is_bound_to_slider(metas, name) {
            MaybeBoundToSlider::Some(meta) => return IsBuiltin::Slider(meta),
            MaybeBoundToSlider::Shadowed => {
                // Slider is shadowed, so it's considered a "normal" var
                return IsBuiltin::None;
            }
            MaybeBoundToSlider::None => (),
        }
        self.builtin_vars
            .get(&name.to_ascii_lowercase())
            .map_or(IsBuiltin::None, IsBuiltin::BuiltIn)
    }

    pub fn add_access(&mut self, access: &access::Undetermined, section_kind: &'static str) {
        let to_add = access::TopLevel {
            origin: access.origin.clone(),
            info: access.info.clone(),
            section: section_kind,
        };
        let name = &to_add.info.accessed_as;
        self.variables
            .entry(name.to_lower().to_string())
            .or_insert_with(|| Variable::new(name))
            .accesses
            .push(to_add);
    }
}
