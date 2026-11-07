use crate::datastructures::statement::DType;

pub mod statement;
pub mod basicblock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeStackEntry {
    Known(DType),
    Depends(usize),
    Unknown
}

pub type TypeStack = Vec<TypeStackEntry>;
