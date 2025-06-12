use crate::access::Kind;

impl Kind {
    pub fn is_equivalent(&self, other: &Self) -> bool {
        // PassedByRef is not considered
        match (self, other) {
            (Self::Read, Self::Read) => true,
            (
                Self::Write { value, potential },
                Self::Write {
                    value: other_value,
                    potential: other_potential,
                },
            ) => value == other_value && potential == other_potential,
            _ => false,
        }
    }
}
