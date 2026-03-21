# Changelog

## 0.1.0 — 2026-03-20

### Added
- Initial release with five device architectures
- NvidiaGPU: SMs + HBM + L2 + GigaThread Engine
- AmdGPU: CUs in Shader Engines + Infinity Cache
- GoogleTPU: Scalar/Vector/MXU pipeline + HBM
- IntelGPU: Xe-Cores in Xe-Slices + L2
- AppleANE: NE cores + SRAM + DMA + unified memory (zero-copy)
- SimpleGlobalMemory with coalescing, partitioning, bandwidth modeling
- GPU/TPU/ANE work distributors
- KernelDescriptor for GPU-style and dataflow-style launches
- DeviceTrace and DeviceStats for observability
