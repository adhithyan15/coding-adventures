# coding_adventures_device_simulator

Complete accelerator device simulators -- Layer 6 of the accelerator computing stack.

## What Is This?

This gem simulates **complete accelerator devices**, assembling multiple compute units (Layer 7) with global memory, L2 cache, and work distribution into full devices that can launch and execute kernels.

Think of it as the difference between simulating one factory floor (Layer 7, compute unit) versus simulating the entire factory complex (Layer 6, device).

## Five Device Simulators

| Device | Architecture | Work Distribution | Memory |
|--------|-------------|-------------------|--------|
| `NvidiaGPU` | SMs + GigaThread Engine | Round-robin block dispatch | HBM + L2 |
| `AmdGPU` | CUs in Shader Engines | Command Processor + ACEs | GDDR + Infinity Cache |
| `GoogleTPU` | Scalar/Vector/MXU pipeline | TPU Sequencer (tiling) | HBM |
| `IntelGPU` | Xe-Cores in Xe-Slices | Command Streamer | GDDR + L2 |
| `AppleANE` | NE cores + DMA | Compiler schedule replay | Unified memory (zero-copy) |

## Where It Fits

```
Layer 9:  gpu-core (one core, one instruction at a time)
    |
Layer 8:  parallel-execution-engine (warps, wavefronts, systolic arrays)
    |
Layer 7:  compute-unit (SM, CU, MXU, XeCore, ANECore)
    |
Layer 6:  device-simulator (THIS PACKAGE)
```

## Usage

### GPU-style (NVIDIA, AMD, Intel)

```ruby
require "coding_adventures_device_simulator"
include CodingAdventures

gpu = DeviceSimulator::NvidiaGPU.new(num_sms: 4)

# Allocate and copy data
addr = gpu.malloc(1024)
gpu.memcpy_host_to_device(addr, "\x00".b * 1024)

# Launch kernel
gpu.launch_kernel(DeviceSimulator::KernelDescriptor.new(
  name: "saxpy",
  program: [GpuCore.limm(0, 2.0), GpuCore.halt],
  grid_dim: [4, 1, 1],
  block_dim: [32, 1, 1],
))

# Run to completion
traces = gpu.run(1000)
puts "Completed in #{traces.length} cycles"
puts traces.last.format
```

### Dataflow-style (TPU, ANE)

```ruby
tpu = DeviceSimulator::GoogleTPU.new(mxu_size: 4)

tpu.launch_kernel(DeviceSimulator::KernelDescriptor.new(
  name: "matmul",
  operation: "matmul",
  input_data: [[1.0, 2.0], [3.0, 4.0]],
  weight_data: [[5.0, 6.0], [7.0, 8.0]],
))

traces = tpu.run(500)
puts "TPU completed in #{traces.length} cycles"
```

### Apple ANE (zero-copy unified memory)

```ruby
ane = DeviceSimulator::AppleANE.new(num_cores: 4)

addr = ane.malloc(256)
cycles = ane.memcpy_host_to_device(addr, "\x42".b * 256)
# cycles == 0, because unified memory is zero-copy!

ane.launch_kernel(DeviceSimulator::KernelDescriptor.new(
  name: "inference",
  operation: "matmul",
  input_data: [[1.0, 2.0], [3.0, 4.0]],
  weight_data: [[5.0, 6.0], [7.0, 8.0]],
))

traces = ane.run(500)
```

## Dependencies

- `coding_adventures_gpu_core` -- Layer 9 (single core)
- `coding_adventures_parallel_execution_engine` -- Layer 8 (warps, wavefronts)
- `coding_adventures_compute_unit` -- Layer 7 (SM, CU, MXU, XeCore, ANECore)
- `coding_adventures_clock` -- Clock generation
- `coding_adventures_cache` -- Cache simulation
- `coding_adventures_fp_arithmetic` -- IEEE 754 floating point

## Development

```bash
bundle install
bundle exec rake test
```

## License

MIT
