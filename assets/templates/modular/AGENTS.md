# Project Guidelines

> Generated from kimi-dotfiles/templates/modular
> Includes: base@v1.5.0 + rust@v1.3.0
>
> <!-- Strictness: {STRICTNESS} -->

<!-- PART: meta -->

## 0. Meta Principle

### 0.1: Types Prove Invariants

Rust's type system is a theorem prover. Use it to prove correctness at compile time. What cannot be proven by types must be proven by property tests.

### 0.2: Every Change Solves a Problem

Before adding, removing, or refactoring anything, ask: **what problem does this solve?**

- **GOOD:** "Raw `u32` for sample rate lets callers pass 0, which crashes the resampler silently. `SampleRate` newtype rejects 0 at the API boundary."
- **BAD:** "Let's add a newtype here because the checklist says so" — ceremony without boundary.
- **BAD:** "The score will go up" — optimizing a metric instead of the code.
- **BAD:** "It's cleaner" — subjective taste without a concrete failure mode prevented.

If the answer is "it looks better" or "the score goes up" — it's not engineering, it's decoration. Revert.

<!-- PART: types -->

## I. Types as Axioms

### Rule 1.1: Newtype for Every Semantic Distinction

Never use raw primitives where meaning matters.

```rust
// AXIOM: Price >= 0
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(u64); // cents

// AXIOM: TaxRate in [0.0, 1.0]
pub struct TaxRate(f64);

impl TaxRate {
    /// { rate >= 0.0 && rate <= 1.0 }
    /// fn new(rate: f64) -> Option<TaxRate>
    /// { Some(_) ==> rate in [0,1], None ==> rate outside [0,1] }
    pub fn new(rate: f64) -> Option<Self> {
        if rate >= 0.0 && rate <= 1.0 { Some(Self(rate)) } else { None }
    }
}
```

### Rule 1.2: Phantom Types for Dimensions

```rust
use std::marker::PhantomData;

pub struct Meters;
pub struct Seconds;

#[derive(Clone, Copy, Debug)]
pub struct Quantity<T>(f64, PhantomData<T>);

impl Quantity<Meters> {
    pub fn meters(v: f64) -> Self { Self(v, PhantomData) }
    pub fn value(&self) -> f64 { self.0 }
}
```

### Rule 1.3: Typestate for State Machines

```rust
use std::marker::PhantomData;

pub struct Disconnected;
pub struct Connected;

pub struct Session<State> {
    socket: std::net::TcpStream,
    _state: PhantomData<State>,
}

impl Session<Disconnected> {
    pub fn connect(addr: &str) -> std::io::Result<Session<Connected>> {
        let socket = std::net::TcpStream::connect(addr)?;
        Ok(Session { socket, _state: PhantomData })
    }
}
```

### Rule 1.4: Newtype Is a Boundary, Not a Costume

A newtype is justified only when it protects a module boundary. Wrapping and immediately unwrapping in the same scope is **ceremony** — excluded by the mathematical approach.

```rust
// BAD: ceremony. The newtype is created and unwrapped in the same breath.
pub fn process(v: u32) {
    let rate = Rate::new(v).unwrap_or(Rate(1)); // unwrap_or hides logic
    ...
}

// GOOD: boundary is real. The caller passes a validated Rate.
pub fn process(rate: Rate) { ... }

// GOOD: internal code that already proved the invariant uses from_raw.
pub(crate) fn from_raw(v: u32) -> Self {
    debug_assert!(v > 0, "Rate invariant violated");
    Rate(v)
}
```

**Guidelines:**
- `new` → for external API (validation, `Result`/`Option`).
- `from_raw` → `pub(crate)` escape hatch for internal code that already guaranteed the invariant.
- Never `Type::new(x).unwrap_or(...)` in the same function that computes `x`.

<!-- PART: functions -->

## II. Functions as Lemmas: Hoare Logic

### Rule 2.1: Every Public Function Has a Hoare Triple

```rust
/// { !items.is_empty() }
/// fn average(items: &[f64]) -> f64
/// { ret.abs_diff_eq(sum(items) / len(items), epsilon) }
pub fn average(items: &[f64]) -> f64 {
    debug_assert!(!items.is_empty());
    items.iter().sum::<f64>() / items.len() as f64
}
```

### Rule 2.2: Precondition via debug_assert!

Use `debug_assert!` for preconditions the type system cannot enforce. Zero cost in release.

### Rule 2.3: Function Length ≤ 40 Lines

A lemma must fit in working memory. If longer — decompose into sub-lemmas.

<!-- PART: algebra -->

## III. Algebraic Structures as Traits

### Rule 3.1: Mathematical Structures Are Contracts with Axioms

```rust
/// Axiom: ∀a,b,c. combine(a, combine(b, c)) == combine(combine(a, b), c)
pub trait Semigroup: Clone + PartialEq {
    fn combine(&self, other: &Self) -> Self;
}
```

### Rule 3.2: Axioms MUST Be Verified by Property Tests

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn associativity(a in any::<MyMonoid>(), b in any::<MyMonoid>(), c in any::<MyMonoid>()) {
            let left = a.combine(&b.combine(&c));
            let right = a.combine(&b).combine(&c);
            assert_eq!(left, right);
        }
    }
}
```

<!-- PART: errors -->

## IV. Error Handling: Errors as Codomain

### Rule 4.1: Result/Option Instead of Panic

`unwrap()`, `expect()`, `panic!` — forbidden in library code without compile-time proof.

```rust
// FORBIDDEN
let port = env::var("PORT").unwrap().parse::<u16>().unwrap();

// REQUIRED
let port: NonZeroU16 = env::var("PORT")
    .map_err(|e| Error::ConfigMissing("PORT", e))?
    .parse()
    .map_err(Error::InvalidPort)?;
```

<!-- PART: iterators -->

## V. Iterators: Composition as Proof

### Rule 5.1: Iterator Chains > Nested Control Flow

```rust
// BAD: Kimi gets lost in brackets
let x = match maybe_y {
    Some(y) => match y.parse::<i32>() {
        Ok(z) if z > 0 => Some(z * 2),
        _ => None,
    },
    None => None,
};

// GOOD: compositional, each step is a lemma
let x = maybe_y
    .and_then(|y| y.parse::<i32>().ok())
    .filter(|&z| z > 0)
    .map(|z| z * 2);
```

<!-- PART: unsafe -->

## VI. Unsafe: Proof Obligation

### Rule 6.1: Every unsafe Block Requires a Proof

```rust
// SAFETY: ptr is valid because:
// 1. Allocated via Box::into_raw at line 42.
// 2. len matches allocation size (invariant of Self).
// 3. ptr is not aliased (exclusive borrow).
unsafe { std::slice::from_raw_parts_mut(ptr, len) }
```

### Rule 6.2: Miri Check Mandatory

```bash
cargo +nightly miri test
```

<!-- PART: testing -->

## VII. Testing: Universal Quantification

### Rule 7.1: Property Tests Are Mandatory for Algebraic APIs

Every `Semigroup`, `Monoid`, `Functor` implementation MUST have property tests.

### Rule 7.2: Doc Tests Are Executable Theorems

Every public function MUST have a doc test.

<!-- PART: automation -->

## VIII. Automation

### Rule 8.1: rustfmt and clippy Mandatory

```toml
[lints.clippy]
all = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

### Rule 8.2: CI for Every PR

```bash
cargo check --all-features
cargo clippy -- -D warnings
cargo test
cargo doc --no-deps
```

### Rule 8.3: Mechanized Contract Checks

Run `cargo kimi check` before every commit to verify:
- Every `pub fn` has a Hoare triple doc comment
- No `unwrap()` / `expect()` / `panic!()` outside tests
- Every `unsafe` block has a `// SAFETY:` comment

<!-- PART: checklist -->

## IX. Checklist

- [ ] Change is justified: it prevents a concrete bug or clarifies an invariant
- [ ] Hoare triple in doc comment
- [ ] Invariant in type (Newtype/Phantom/Typestate) or `debug_assert!`
- [ ] No unwrap/expect/panic without compile-time proof
- [ ] Property tests for algebraic axioms
- [ ] Doc tests for examples
- [ ] unsafe has SAFETY proof + Miri check
- [ ] Function ≤ 40 lines
- [ ] No nesting > 3 levels
- [ ] clippy passes without warnings
- [ ] All tests pass (unit + property + doc)
- [ ] `cargo kimi check` passes

<!-- PART: project-specific -->

## X. Project-Specific Rules

<!-- Add your project conventions here -->
