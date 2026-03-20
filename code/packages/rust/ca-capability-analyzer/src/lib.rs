//! # ca-capability-analyzer — Static Capability Analyzer for Rust
//!
//! This crate performs **static analysis** on Rust source code to detect
//! OS-level capability usage. It answers the question: *"What external
//! resources does this Rust code access?"*
//!
//! ## The Problem
//!
//! A Rust package might read files, open network sockets, spawn processes,
//! or call into C libraries — all without declaring these capabilities
//! anywhere. A library that only does math *shouldn't* need filesystem
//! access, but nothing enforces this today.
//!
//! ## The Approach
//!
//! We parse Rust source files into an AST using the [`syn`] crate, then
//! walk the tree looking for patterns that indicate OS capability usage.
//! Each pattern maps to a **capability triple**: `category:action:target`.
//!
//! For example:
//!
//! ```text
//! use std::fs;           →  fs:*:*     (broad filesystem access)
//! File::open("data.txt") →  fs:read:data.txt
//! TcpStream::connect(..) →  net:connect:*
//! Command::new("ls")     →  proc:exec:ls
//! env::var("HOME")       →  env:read:HOME
//! unsafe { ... }         →  ffi:*:*    (potential FFI / memory safety)
//! ```
//!
//! ## Design Decisions
//!
//! 1. **`syn` for parsing** — `syn` is the de facto Rust AST parser. It
//!    gives us typed nodes (`ItemUse`, `ExprCall`, `ExprMethodCall`) that
//!    we can pattern-match against. We use the `Visit` trait for tree walking.
//!
//! 2. **Delegate to Clippy where possible** — For banned constructs like
//!    `unsafe` blocks, Clippy already has excellent lints. Our analyzer
//!    focuses on *capability detection* (mapping code patterns to
//!    category:action:target) and *manifest comparison* (checking declared
//!    vs actual capabilities).
//!
//! 3. **Conservative detection** — When we can't determine a target
//!    statically (e.g., `File::open(some_variable)`), we record `*` as
//!    the target. False positives are preferable to false negatives in
//!    a security context.
//!
//! 4. **fnmatch-style glob matching** — When comparing detected targets
//!    against declared targets in the manifest, we use glob patterns.
//!    This mirrors OpenBSD's `unveil()` approach.
//!
//! ## Modules
//!
//! - [`analyzer`] — Core AST walker and capability detection
//! - [`manifest`] — Manifest loading and capability comparison
//! - [`cli`] — Command-line interface

pub mod analyzer;
pub mod cli;
pub mod manifest;
