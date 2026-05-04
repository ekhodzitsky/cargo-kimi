# Project Guidelines

> Generated from kimi-dotfiles/templates/minimal
> Version: 1.4.0
>
> <!-- Strictness: standard -->

## Core Principles

- **Types as axioms** — encode invariants in the type system (Newtype, PhantomData, Typestate)
- **Functions as lemmas** — every public function has a Hoare triple in its doc comment
- **No unwrap/expect/panic** without compile-time proof of safety
- **Property tests** for all algebraic structures (associativity, identity, etc.)
- **Standard patterns** — avoid custom DSLs and macros

## Hoare Triple Template

```rust
/// { precondition }
/// fn name(args) -> ReturnType
/// {
///   Ok(r)  ==> postcondition for success
///   Err(e) ==> postcondition for error
/// }
```

## Error Handling

- `Result` / `Option` everywhere
- Typed errors, not strings
- All error paths tested

## Testing

- Unit tests for examples
- Property tests (proptest) for universal properties
- Doc tests as executable theorems
