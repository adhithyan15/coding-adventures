# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- RegisterFile with read/write/dump, bit-width masking, and bounds checking
- Memory with byte and word read/write, little-endian byte order, load_bytes, dump
- CPU with fetch-decode-execute pipeline, step/run methods, and PipelineTrace
- Immutable Data.define records: FetchResult, DecodeResult, ExecuteResult, PipelineTrace
- PipelineTrace#format_pipeline for visual pipeline diagrams
- Knuth-style literate comments explaining CPU architecture concepts
