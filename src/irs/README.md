This directory contains the definitions for the compiler's various **I**ntermediate **R**epresentations.
There are not many:
- `token.rs` is the token-based IR produced by the tokenizer
- `stack.rs` is the stack-machine-like IR produced by the parser from the output of the tokenizer.
    It is used for flow- and type-checking.
- `register.rs` is the infinite-register-machine form IR produced by the register translator from the stack IR.
    It is used for register allocation and eventually code generation.
