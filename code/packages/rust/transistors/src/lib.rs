//! # Transistors — the foundation of all digital hardware.
//!
//! A transistor is a tiny electronic switch that can be controlled by voltage
//! (MOSFET) or current (BJT). Every digital circuit ever built — from the
//! Apollo Guidance Computer to a modern 100-billion-transistor GPU — is
//! constructed from transistors arranged into logic gates.
//!
//! This crate provides:
//!
//! - **`types`** — shared enums (operating regions), parameter structs, and
//!   result types used throughout the crate.
//! - **`mosfet`** — NMOS and PMOS field-effect transistors, the building
//!   blocks of CMOS logic.
//! - **`bjt`** — NPN and PNP bipolar junction transistors, the building
//!   blocks of TTL logic and analog amplifiers.
//! - **`cmos_gates`** — complete CMOS logic gates (NOT, NAND, NOR, AND, OR,
//!   XOR) built from NMOS/PMOS pairs.
//! - **`ttl_gates`** — historical TTL NAND gate and RTL inverter built from
//!   NPN transistors.
//! - **`amplifier`** — analog amplifier analysis for common-source (MOSFET)
//!   and common-emitter (BJT) topologies.
//! - **`analysis`** — noise margins, power consumption, timing, CMOS vs TTL
//!   comparison, and technology scaling demonstrations.

pub mod types;
pub mod mosfet;
pub mod bjt;
pub mod cmos_gates;
pub mod ttl_gates;
pub mod amplifier;
pub mod analysis;
