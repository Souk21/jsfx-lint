use crate::rcsubstring::RcSubString;

#[derive(Debug)]
pub enum VarKind {
    TempString,
    This {
        /// Suffix of the variable after the "this." prefix.
        /// Examples:
        ///  - `this.foo` -> `Some("foo")`
        ///  - `this` -> None
        ///  - `this..foo` -> `Some(".foo")`
        suffix: Option<RcSubString>,
    },
    Instance {
        /// Suffix of the variable after the instance name without the dot.
        /// Examples:
        /// - `instance(foo)` and `foo` -> `None`
        /// - `instance(foo)` and `foo.bar` -> `Some("bar")`
        /// - `instance(foo)` and `foo.inner.bar` -> `Some("inner.bar")`
        suffix: Option<RcSubString>,
    },
    RefArg {
        /// Suffix of the variable after the argument name, with the dot.
        suffix: RcSubString,
        arg_index: usize,
    },
    Arg {
        arg_index: usize,
    },
    Local,
    Global {
        accessible: bool,
    },
}
