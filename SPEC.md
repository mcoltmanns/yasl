# yasl specification

Conceptually, yasl emits code for a two-stack machine.
The data stack is where general program data lives.
The call stack is where procedure call information lives.

Programs consist of a series of statements.
Statements are separated by newlines (\n).
Lines starting with // are treated as comments and ignored.

## Types
- i8 i16 i32 i64
- u8 u16 u32 u64
- f16 f32 f64
- ptr

### Casting
- cast <type>
    - reinterpret the top of the stack as the given type (no conversion)
- conv <type>
    - convert the top of the stack to the given type (may truncate or extend)

### Comments
Comments start with a // and run to the newline.

### Constants
Constants can be used to define typed literal values.
These are substituted at compile time.
- const <name> <type> <literal>

## Instructions
All instructions consume their operands (values on the stack on which they operate) and place their result (if they have one) on the top of the stack.

### Stack control
- push <type> <literal>
    - put a literal on top of the stack
- pop
    - drop the top of the stack
- dup
    - duplicate the top of the stack
- swap
    - swap the top two values on the stack

### Operations
Operations consume values on the top of the stack. If they return a value, they leave it on the top.
For example:
```
push 3
push 5
sub
```
will leave 2 at the top of the stack (note argument order - rightmost first).
#### Math
- add
- sub
- mul
- div
- mod
- inc
- dec
#### Bitwise
- and
- or
- xor
- bsl
- bsr
- rol
- ror
#### Comparative
- eq
- neq
- lt
- leq
- gt
- geq

### Memory control
- load <type>
    - load the value pointed to by the top of the stack as <type>
- store <type>
    - store the value at the top of the stack to the location at the second position in the stack as <type>

### Control flow
- label <name>
    - define a label to jump to. Labels must be globally unique.
- jump <name>
    - jump to a label. Jumps may be forward or backward.
- jumpif <name>
    - conditional jump. Consumes top, and jumps if not 0.
- call <name>
    - push the current execution location to the call stack and jump to the label.
- ret
    - pop the call stack and return to that execution location.
