# Rust Patterns Used in This Project

This document explains the Rust language features and patterns used
throughout the coding-adventures Rust packages. Rust brings memory safety
without garbage collection — the compiler enforces ownership rules at
compile time.

## Ownership and Borrowing — Rust's Core Idea

Every value in Rust has exactly one owner. When the owner goes out of
scope, the value is dropped (freed). You can lend values via references:

```rust
fn count_ones(bits: &[u8]) -> usize {
    //                ^ borrowed (read-only reference)
    bits.iter().filter(|&&b| b == 1).count()
}

let my_bits = vec![1, 0, 1, 1];
let count = count_ones(&my_bits);  // borrow, don't transfer ownership
// my_bits is still usable here
```

The `&` means "I'm borrowing this, not taking ownership." The compiler
guarantees no one modifies `bits` while it's borrowed. This prevents
data races, use-after-free, and dangling pointers — all at compile time.

**Where used:** Every Rust package

## Structs and Impl Blocks

Rust uses structs for data and `impl` blocks for methods:

```rust
/// A single logic gate output: a bit that is always 0 or 1.
pub struct Bit(u8);

impl Bit {
    pub fn new(value: u8) -> Self {
        assert!(value <= 1, "Bit must be 0 or 1, got {}", value);
        Bit(value)
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}
```

The `self` parameter is like Python's `self` or Ruby's implicit receiver,
but with explicit borrowing: `&self` borrows, `&mut self` borrows mutably,
`self` takes ownership.

**Where used:** `code/packages/rust/logic-gates/`, `code/packages/rust/arithmetic/`

## Traits — Rust's Interfaces

Traits define shared behavior, like Go interfaces or Python Protocols:

```rust
use std::ops::{BitAnd, BitOr, BitXor, Not};

impl BitAnd for Bit {
    type Output = Bit;

    fn bitand(self, rhs: Bit) -> Bit {
        Bit(self.0 & rhs.0)
    }
}
```

Implementing standard library traits like `BitAnd`, `Display`, `From`
lets your types work with Rust's operators and formatting:

```rust
let a = Bit::new(1);
let b = Bit::new(1);
let result = a & b;  // calls BitAnd::bitand
println!("{}", result);  // calls Display::fmt
```

**Where used:** `code/packages/rust/logic-gates/`, `code/packages/rust/arithmetic/`

## Derive Macros — Auto-Generated Implementations

The `#[derive]` attribute generates trait implementations automatically:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CacheLine {
    tag: u64,
    data: Vec<u8>,
    valid: bool,
    dirty: bool,
}
```

- `Debug` — enables `{:?}` formatting for debugging
- `Clone` — enables `.clone()` to copy the value
- `PartialEq` — enables `==` comparison
- `Serialize`/`Deserialize` (from serde) — enables JSON serialization

**Where used:** Every Rust package

## Pattern Matching with `match`

Rust's `match` is exhaustive — the compiler forces you to handle every case:

```rust
enum ALUOp {
    Add, Sub, And, Or, Xor, Not,
}

fn execute(op: ALUOp, a: u32, b: u32) -> u32 {
    match op {
        ALUOp::Add => a.wrapping_add(b),
        ALUOp::Sub => a.wrapping_sub(b),
        ALUOp::And => a & b,
        ALUOp::Or  => a | b,
        ALUOp::Xor => a ^ b,
        ALUOp::Not => !a,
    }
    // No default needed — compiler verifies all variants are covered.
    // Adding a new variant to ALUOp would cause a compile error here.
}
```

**Where used:** `code/packages/rust/arithmetic/`, `code/packages/rust/cache/`

## Result and Option — No Null, No Exceptions

Rust has no null and no exceptions. Instead:

- `Option<T>` = either `Some(value)` or `None`
- `Result<T, E>` = either `Ok(value)` or `Err(error)`

```rust
fn get_build_file(directory: &Path) -> Option<PathBuf> {
    let build = directory.join("BUILD");
    if build.is_file() {
        Some(build)
    } else {
        None
    }
}

// The caller MUST handle the None case:
match get_build_file(dir) {
    Some(path) => println!("Found: {}", path.display()),
    None => println!("No BUILD file"),
}
```

The `?` operator propagates errors concisely:

```rust
fn read_and_parse(path: &Path) -> Result<Config, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;  // returns Err if read fails
    let config: Config = toml::from_str(&text)?;  // returns Err if parse fails
    Ok(config)
}
```

**Where used:** `code/programs/rust/build-tool/`

## Iterators and Closures

Rust iterators are zero-cost abstractions — they compile to the same
machine code as hand-written loops:

```rust
// Count how many packages need rebuilding
let needs_build_count = packages.iter()
    .filter(|pkg| cache.needs_build(&pkg.name, &pkg.hash))
    .count();

// Collect package names into a sorted Vec
let mut names: Vec<&str> = packages.iter()
    .map(|pkg| pkg.name.as_str())
    .collect();
names.sort();
```

Common iterator methods: `.map()`, `.filter()`, `.collect()`, `.fold()`,
`.any()`, `.all()`, `.find()`, `.enumerate()`, `.zip()`.

**Where used:** Every Rust package

## `#[cfg(test)]` — Conditional Compilation for Tests

Tests live in the same file as the code, inside a `#[cfg(test)]` module:

```rust
pub fn half_adder(a: u8, b: u8) -> (u8, u8) {
    (a ^ b, a & b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_half_adder_truth_table() {
        assert_eq!(half_adder(0, 0), (0, 0));
        assert_eq!(half_adder(0, 1), (1, 0));
        assert_eq!(half_adder(1, 0), (1, 0));
        assert_eq!(half_adder(1, 1), (0, 1));
    }
}
```

The `#[cfg(test)]` attribute means this module is only compiled when
running `cargo test`. It's stripped from release builds.

**Where used:** Every Rust package

## Rayon — Parallel Iterators

The rayon crate adds `.par_iter()` for parallel iteration:

```rust
use rayon::prelude::*;

// Build packages in parallel — rayon handles work-stealing
packages.par_iter().for_each(|pkg| {
    build_package(pkg);
});
```

Switching from `.iter()` to `.par_iter()` is often the only change needed.
Rayon handles thread pool management, work stealing, and load balancing.

**Where used:** `code/programs/rust/build-tool/src/executor.rs`

## Modules and Visibility

Rust's module system maps to the file system:

```
src/
├── main.rs          mod discovery;  // declares the module
├── discovery.rs     pub fn discover_packages(...)  // public function
├── resolver.rs      pub(crate) fn resolve(...)     // crate-internal
└── hasher.rs        fn hash_file(...)              // private (default)
```

- `pub` — visible to everyone
- `pub(crate)` — visible within the crate only
- no modifier — private to the module

**Where used:** `code/programs/rust/build-tool/`

## Lifetimes — Borrowed Data Must Outlive Its Borrow

When a function returns a reference, Rust needs to know how long it lives:

```rust
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() > b.len() { a } else { b }
}
```

The `'a` annotation says "the returned reference lives as long as the
shortest-lived input." The compiler uses this to prevent dangling references
at compile time.

In practice, you rarely write explicit lifetimes — the compiler infers them
in most cases (lifetime elision rules).

**Where used:** Various internal functions
