# yasl specification

yasl is a low-level stack language which compiles to register form.
It is designed to be simple and portable (not necessarily fast).
Nearly all operations consume from and push to the data stack, which is strictly typed.
Strict compile-time checks make stack underflows and type errors at runtime impossible.
During compilation, the stack is discarded entirely and the program is translated to a traditional register architecture.
This increases speed and eliminates the need for an interpreter or virtual machine on systems with limited resources.

## Basic syntax
Programs consist of a series of statements/instructions.
Statements are separated by newlines (\n).
Lines starting with // are treated as comments and ignored.

## Mandatory procedures
All programs must have an entry procedure `main`.
`main` may have arguments, but the data in them is not guaranteed to exist or be valid at runtime since it comes from a context the compiler cannot control.
Arguments to `main` are used during compile time analysis to ensure stack and type correctness.

It is possible, but not required, to define an interrupt handling procedure `trapper`.
`trapper` is executed whenever an interrupt is triggered, be it in software or in hardware.
`trapper` must be zero-effect, meaning it cannot expect or leave values on the stack.
However, it is possible to save and load data to and from heap memory with the `store` and `load` instructions.
Although yasl defines a `trap` instruction which triggers a maskable software interrupt, it is actually faster to call the procedure directly if possible, since this avoids the full context switch induced by a real interrupt.
All interrupts are maskable and masking.
The exception to this rule is a non-maskable hardware interrupt (such as falling edge NMI on 6502), but yasl provides no mechanism to handle these (yet).

## Types
- i8 i16 i32 i64
- u8 u16 u32 u64
- f16 f32 f64
- ptr

### Casting
- cast \<type>
    - reinterpret the top of the stack as the given type (no conversion, bits are reinterpreted/value may change)
- conv \<type>
    - convert the top of the stack to the given type (may truncate or extend, bits are converted/value is kept or wrapped)

### Comments
Comments start with a // and run to the newline.

### Constants
Constants can be used to define typed literal values.
These are substituted at compile time.
- const \<name> \<type> \<literal>

## Instructions
All instructions consume their operands (values on the stack on which they operate) and place their result (if they have one) on the top of the stack.

### Stack control
- push \<type> \<literal>
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
- not
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
#### Memory control
Careful! These also consume data on the stack.
E.g: stack = 1 -> load u8 -> stack = \<data at addr 1>. Or stack = 1 2 -> store u8 -> memory 2 contains 1
- load \<type>
    - load the value pointed to by the top of the stack as \<type> onto the top of the stack
- store \<type>
    - store the value at the top of the stack to the location at the second position in the stack

### Control flow
- label \<name>
    - define a label to jump to. Labels must be globally unique.
- proc \<name> in \<type1> ... \<typeN> out \<type1> ... \<typeN> def
    - define a procedure with guaranteed input and output types.
- jump \<name>
    - jump to a label. Jumps may be forward or backward, but cannot cross procedure boundaries.
- jumpif \<name>
    - conditional jump. Consumes top, and jumps if not 0.
    - jumpif can only operate on integer types (i or u)
- call \<name>
    - call a procedure. Control is returned to the caller afterwards.
    - when a procedure begins execution, its stack will contain only the types defined in its signature.
- ret
    - return from a procedure (to the caller).
    - when ret is called, a procedure's data stack must contain only the types defined in its signature.

### Interrupts
- trap
  - trigger an interrupt (but it's usually faster to just call your trap procedure directly)
