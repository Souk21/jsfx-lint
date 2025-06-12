use crate::access::{Origin, OriginDetails};
use uuid::Uuid;

impl Origin {
    pub fn or_if_undetermined<F>(&self, origin_fn: F) -> Self
    where
        F: FnOnce() -> Self,
    {
        match self {
            Self::Undetermined => origin_fn(),
            _ => self.clone(),
        }
    }

    pub const fn get_uuid(&self) -> Option<&Uuid> {
        match self {
            Self::This(OriginDetails { uuid, .. })
            | Self::Instance(OriginDetails { uuid, .. })
            | Self::Ref(OriginDetails { uuid, .. }) => Some(uuid),
            _ => None,
        }
    }
}
