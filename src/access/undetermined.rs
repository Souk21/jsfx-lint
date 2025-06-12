use crate::access;
use crate::access::var_kind::VarKind;
use crate::access::{Info, Origin, OriginDetails, Undetermined, WithinFunction};
use crate::functions::Fun;
use uuid::Uuid;

impl Undetermined {
    pub fn to_within_function(&self, fun: &Fun) -> WithinFunction {
        let var_kind = if self.force_global_scope {
            let accessible =
                self.bypass_global_modifier || fun.global_var_is_accessible(&self.info.accessed_as);
            VarKind::Global { accessible }
        } else {
            fun.classify_variable(&self.info.accessed_as, self.bypass_global_modifier)
        };
        let uuid = Uuid::new_v4();
        let origin = match var_kind {
            VarKind::TempString | VarKind::Arg { .. } | VarKind::Local | VarKind::Global { .. } => {
                self.origin.clone()
            }
            VarKind::This { .. } => self.origin.or_if_undetermined(|| {
                Origin::This(OriginDetails {
                    uuid,
                    global_scope_navigation_override: None,
                })
            }),
            VarKind::Instance { .. } => self.origin.or_if_undetermined(|| {
                Origin::Instance(OriginDetails {
                    uuid,
                    global_scope_navigation_override: None,
                })
            }),
            VarKind::RefArg { .. } => self.origin.or_if_undetermined(|| {
                Origin::Ref(OriginDetails {
                    uuid,
                    global_scope_navigation_override: None,
                })
            }),
        };
        WithinFunction {
            info: self.info.clone(),
            var_kind,
            uuid,
            origin,
        }
    }
    pub fn to_potential(&self) -> Self {
        let mut new = self.clone();
        if let Self {
            info:
                Info {
                    kind: access::Kind::Write { potential, .. },
                    ..
                },
            ..
        } = &mut new
        {
            *potential = true;
        }
        new
    }
    pub fn is_equivalent(&self, other: &Self) -> bool {
        self.info.accessed_as.to_lower() == other.info.accessed_as.to_lower()
            && self.info.kind.is_equivalent(&other.info.kind)
            && self.force_global_scope == other.force_global_scope
            && self.bypass_global_modifier == other.bypass_global_modifier
    }
}
