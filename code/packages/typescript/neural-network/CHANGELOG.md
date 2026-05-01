# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- Added `createFeedForwardNetwork()` for authoring matrix-shaped feed-forward
  networks from input names, layer weights, biases, activations, and output names.
- Added a `constant` primitive for scalar bias or literal nodes.
- Added `createXorNetwork()` as an explicit hidden-layer graph example.

## [0.1.0] - 2026-04-29

### Added

- Added a generic `NeuralNetwork` model backed by `MultiDirectedGraph<string>`.
- Added primitive helpers for inputs, weighted sums, activations, and outputs.
- Added reserved `nn.*` metadata authoring helpers for VM compiler consumption.
