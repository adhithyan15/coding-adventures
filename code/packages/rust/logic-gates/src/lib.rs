//! # Logic Gates — the foundation of all digital computing.
//!
//! A logic gate is the simplest possible decision-making element. It takes
//! one or two inputs, each either 0 or 1, and produces a single output
//! that is also 0 or 1. The output is entirely determined by the inputs —
//! there is no randomness, no hidden state, no memory.
//!
//! In physical hardware, gates are built from transistors — tiny electronic
//! switches etched into silicon. A modern CPU contains billions of transistors
//! organized into billions of gates. But conceptually, every computation a
//! computer performs — from adding numbers to rendering video to running AI
//! models — ultimately reduces to combinations of these simple 0-or-1 operations.
//!
//! This crate provides:
//! - **`gates`** — the seven fundamental gates (AND, OR, NOT, XOR, NAND, NOR, XNOR),
//!   NAND-derived gates proving functional completeness, and multi-input variants.
//! - **`sequential`** — memory elements (SR latch, D latch, D flip-flop, register,
//!   shift register) that give circuits the ability to remember.

pub mod gates;
pub mod sequential;
