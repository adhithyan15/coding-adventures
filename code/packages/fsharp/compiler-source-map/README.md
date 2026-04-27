# CodingAdventures.CompilerSourceMap.FSharp

Compiler source-map chain sidecar for .NET.

The package models source positions, source-to-AST mappings, AST-to-IR mappings, optimizer IR-to-IR mappings, and IR-to-machine-code mappings. `SourceMapChain` composes those segments for forward source-to-machine-code lookups and reverse machine-code-to-source lookups.
