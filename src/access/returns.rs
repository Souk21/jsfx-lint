use crate::access;
use crate::access::{
    Info, Origin, Return, ReturnKind, Undetermined, get_value_from_previous_accesses,
};
use crate::value::Value;

impl Return {
    pub(super) fn is_equivalent(&self, other: &Self) -> bool {
        match &self.kind {
            ReturnKind::Named(name) => match &other.kind {
                ReturnKind::Named(other_name) => {
                    name.to_lower() == other_name.to_lower() && self.potential == other.potential
                }
                ReturnKind::Value(_) => false,
            },
            ReturnKind::Value(value) => match &other.kind {
                ReturnKind::Value(other_value) => {
                    value == other_value && self.potential == other.potential
                }
                ReturnKind::Named(_) => false,
            },
        }
    }

    pub fn named_to_value(&self, accesses: &Vec<Undetermined>) -> Self {
        match &self.kind {
            ReturnKind::Named(name) => {
                let value = get_value_from_previous_accesses(name, accesses);
                Self {
                    kind: ReturnKind::Value(value),
                    location: self.location.clone(),
                    potential: self.potential,
                }
            }
            ReturnKind::Value(_) => self.clone(),
        }
    }

    pub(super) fn into_potential(self) -> Self {
        Self {
            kind: self.kind,
            potential: true,
            location: self.location,
        }
    }

    pub(super) fn to_read(&self) -> Option<Undetermined> {
        self.to_undetermined(access::Kind::Read)
    }

    pub(super) fn to_passed_by_ref(&self) -> Option<Undetermined> {
        self.to_undetermined(access::Kind::PassedByRef)
    }

    pub(super) fn to_write(&self, value: Value) -> Option<Undetermined> {
        self.to_undetermined(access::Kind::Write {
            value,
            potential: self.potential,
        })
    }

    fn to_undetermined(&self, kind: access::Kind) -> Option<Undetermined> {
        match &self.kind {
            ReturnKind::Named(name) => Some(Undetermined {
                origin: Origin::Undetermined,
                info: Info {
                    location: self.location.clone(),
                    accessed_as: name.clone(),
                    kind,
                },
                force_global_scope: false,
                bypass_global_modifier: false,
            }),
            ReturnKind::Value(_) => None,
        }
    }
}
