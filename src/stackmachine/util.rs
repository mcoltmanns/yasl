use crate::datastructures::statement::DType;

pub enum TypeStackEntry {
    /// A known datatype on the stack
    Known(DType),
    /// A type on the stack that depends on a type at some index from the TOP of the stack
    Depends(usize),
    Unknown
}
pub type TypeStack = Vec<TypeStackEntry>;

/// A StackEffector is an object which has an effect on the stackmachine's tracking stack.
/// It modifies this stack in a given way, and can introduce dependencies and fixpoints to it.
pub trait StackEffector {
    /// The minimum number of items this effector needs on the typestack to work correctly.
    fn pops(&self) -> usize;
    /// The items this effector will leave on the typestack after correct operation.
    /// These may depend on items the effector requires.
    fn pushes(&self) -> &TypeStack;
}