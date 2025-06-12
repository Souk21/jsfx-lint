#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Unknown,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(l0), Self::Number(r0)) => *l0 == *r0,
            _ => false,
        }
    }
}
