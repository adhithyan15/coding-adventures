// LogicGates.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// LogicGates — Module Entry Point
// ============================================================================
//
// This module implements the fundamental building blocks of all digital
// circuits. Every computation a computer performs — from adding numbers to
// running neural networks — ultimately reduces to combinations of these gates.
//
// This is Layer 1 of the computing stack. It depends on the transistors
// package (Layer 0), which provides the CMOS physics that physically
// implement each gate.
//
// # Module Structure
//
//   Gates.swift          — The 7 primitive gates + NAND-derived + multi-input
//   Sequential.swift     — SR latch, D latch, D flip-flop, register, shift
//                          register, counter
//   Combinational.swift  — MUX, DEMUX, decoder, encoder, priority encoder,
//                          tri-state buffer
//
// # Quick Reference
//
//   // Primitive gates
//   try notGate(0)         // → 1
//   try andGate(1, 1)      // → 1
//   try orGate(0, 1)       // → 1
//   try xorGate(1, 0)      // → 1
//   try nandGate(1, 1)     // → 0
//   try norGate(0, 0)      // → 1
//   try xnorGate(1, 1)     // → 1
//
//   // Sequential logic
//   let (q, qBar) = try srLatch(set: 1, reset: 0)
//   let (q, qBar) = try dLatch(data: 1, enable: 1)
//   let (q, qBar, _) = try dFlipFlop(data: 1, clock: 1)
//
//   // Combinational circuits
//   try mux2(d0: 0, d1: 1, sel: 1)          // → 1
//   try decoder(inputs: [1, 0])              // → [0, 0, 1, 0]
//   try triState(data: 1, enable: 0)         // → nil (high-Z)
//
// ============================================================================

// All public types, structs, functions, and enums are defined in the
// companion source files and are automatically part of this module.
// Swift does not require explicit re-exports within a single target.

/// The version of this package.
public let version = "0.1.0"
