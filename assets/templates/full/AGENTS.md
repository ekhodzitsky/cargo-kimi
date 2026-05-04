# Project Guidelines

> Generated from kimi-dotfiles/templates/full
> Includes: base@v1.3.0 + rust@v1.3.0
>
> <!-- Strictness: standard -->

This is the complete ruleset. For a shorter version, use `templates/rust-only/AGENTS.md`.

## Base Rules

- **Types prove invariants** — encode constraints in Newtype / Phantom / Typestate
- **Functions have contracts** — Hoare triple in every doc comment
- **No unwrap/expect/panic** without compile-time proof
- **Property tests** for algebraic axioms (associativity, identity, etc.)
- **Standard patterns** — no custom DSLs

## Rust Rules (Full)

### Types as Axioms

- Newtype for every semantic distinction: `struct Price(u64)` not `u64`
- Phantom types for dimensions: `Quantity<Meters>`
- Typestate for state machines: `Socket<Connected>`
- `NonZeroU32`, `NonZeroU64` where applicable

### Functions as Lemmas

Every public function MUST have a Hoare triple:
```rust
/// { !items.is_empty() }
/// fn average(items: &[f64]) -> f64
/// { ret == sum(items) / items.len() }
```

And `debug_assert!` for preconditions not in types:
```rust
pub fn average(items: &[f64]) -> f64 {
    debug_assert!(!items.is_empty());
    items.iter().sum::<f64>() / items.len() as f64
}
```

### Algebraic Structures

```rust
pub trait Semigroup: Clone + PartialEq {
    /// Axiom: ∀a,b,c. combine(a, combine(b, c)) == combine(combine(a, b), c)
    fn combine(&self, other: &Self) -> Self;
}

pub trait Monoid: Semigroup {
    /// Axiom: ∃e. ∀a. combine(e, a) == a && combine(a, e) == a
    fn identity() -> Self;
}
```

Property tests MUST verify all axioms via `proptest`.

### Error Handling

- `Result<T, Error>` with typed errors
- `?` operator preferred
- No `unwrap` outside `#[cfg(test)]`

### Unsafe

Every `unsafe` block requires `// SAFETY:` proof + Miri check.

### Testing

- `#[cfg(test)]` in same file
- `proptest` for property verification
- Doc tests for examples
- `cargo fuzz` for parsers

### Automation

Clippy configuration is installed based on your chosen strictness level:
- `relaxed` — warnings only, no CI breaks
- `standard` — deny unwrap/panic, warn on others (default)
- `strict` — deny everything

Run: `cargo test`, `cargo clippy -- -D warnings`, `cargo doc --no-deps`
For unsafe: `cargo +nightly miri test`

### Mechanized Checks

Use `scripts/check-contracts.py` from kimi-dotfiles to verify:
- Every `pub fn` has Hoare triple marker `/// {`
- No `unwrap`/`expect`/`panic!` in non-test code
- Every `unsafe` has `// SAFETY:` comment

---

## Project-Specific Rules

<!-- Add your project conventions here -->
