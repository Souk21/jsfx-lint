use crate::access;
use crate::access::Info;

impl Info {
    pub const fn is_read(&self) -> bool {
        matches!(&self.kind, access::Kind::Read)
    }
    pub const fn is_write(&self) -> bool {
        matches!(&self.kind, access::Kind::Write { .. })
    }
}
