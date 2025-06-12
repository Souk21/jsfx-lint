// Operator classification is somewhat unconventional, but it adheres to the original grammar specification.

#[derive(Debug, Eq, PartialEq)]
pub enum CmpOperator {
    /// Represents the `==` operator
    Eq,
    /// Represents the `!=` operator
    Ne,
    /// Represents the `>` operator
    Gt,
    /// Represents the `>=` operator
    Gte,
    /// Represents the `<` operator
    Lt,
    /// Represents the `<=` operator
    Lte,
    /// Represents the `===` operator
    ExactEq,
    /// Represents the `!==` operator
    ExactNe,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ModShiftOperator {
    /// Represents the `<<` operator
    Left,
    /// Represents the `>>` operator
    Right,
    /// Represents the `%` operator
    Mod,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LogicalAndOrOperator {
    /// Represents the `&&` operator
    And,
    /// Represents the `||` operator
    Or,
}

#[derive(Debug, Eq, PartialEq)]
pub enum AndOrOperator {
    /// Represents the `&` operator
    And,
    /// Represents the `|` operator
    Or,
    /// Represents the `~` operator
    Xor,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    /// Represents the `!` operator
    Not,
    /// Represents the `-` operator
    Neg,
    /// Represents the `+` operator
    Pos,
}

#[derive(Debug, Eq, PartialEq)]
pub enum AssignmentOperator {
    /// Represents the `=` operator
    Assign,
    /// Represents the `+=` operator
    Add,
    /// Represents the `-=` operator
    Sub,
    /// Represents the `*=` operator
    Mul,
    /// Represents the `/=` operator
    Div,
    /// Represents the `%=` operator
    Mod,
    /// Represents the `|=` operator
    Or,
    /// Represents the `&=` operator
    And,
    /// Represents the `~=` operator
    Xor,
    /// Represents the `^=` operator
    Pow,
}
