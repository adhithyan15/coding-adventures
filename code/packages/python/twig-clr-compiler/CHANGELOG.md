# Changelog — twig-clr-compiler

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — TW02 v1: arithmetic floor

### Added

- ``compile_to_ir(source)`` — Twig source → ``IrProgram`` for the
  integer-arithmetic subset of TW00 (literals, ``+`` / ``-`` /
  ``*`` / ``/`` / ``=`` / ``<`` / ``>``, ``if`` / ``let`` /
  ``begin``, top-level expression as program result).
- ``compile_source(source)`` — full pipeline: parse + AST extract +
  IR emit + ir-optimizer + ir-to-cil-bytecode + cli-assembly-writer
  → ``PackageResult`` with assembly bytes.
- ``run_source(source)`` — ``compile_source`` then run on
  ``clr-vm-simulator``; returns the program's value.
- TW02 spec: ``code/specs/TW02-twig-clr-compiler.md``.

### Notes

- ``define`` / ``lambda`` / ``cons`` / ``print`` raise
  ``TwigCompileError`` for now — TW02.5 / TW03 add them.
- No tail-call optimisation; deep recursion blows the CLR stack.
