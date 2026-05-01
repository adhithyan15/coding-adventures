import type { AsyncNeuralMatrixBackend } from "./matrix-plan.js";

const GPU_BUFFER_USAGE = {
  MAP_READ: 0x0001,
  COPY_SRC: 0x0004,
  COPY_DST: 0x0008,
  STORAGE: 0x0080,
} as const;

const GPU_MAP_MODE = {
  READ: 0x0001,
} as const;

const GPU_SHADER_STAGE = {
  COMPUTE: 0x0004,
} as const;

const WORKGROUP_SIZE = 64;

export type WebGpuActivationName = "none" | "relu" | "sigmoid" | "tanh";

export interface WebGpuMatrix {
  readonly rows: number;
  readonly cols: number;
  readonly length: number;
  readonly byteLength: number;
  readonly buffer: GpuBufferLike;
  readonly scratch?: readonly GpuBufferLike[];
}

export interface WebGpuMatrixBackendOptions {
  readonly powerPreference?: "low-power" | "high-performance";
}

export interface WebGpuLike {
  requestAdapter(
    options?: WebGpuMatrixBackendOptions
  ): Promise<GpuAdapterLike | null> | GpuAdapterLike | null;
}

interface GpuAdapterLike {
  requestDevice(): Promise<GpuDeviceLike> | GpuDeviceLike;
}

interface GpuQueueLike {
  writeBuffer(
    buffer: GpuBufferLike,
    bufferOffset: number,
    data: ArrayBuffer | ArrayBufferView,
    dataOffset?: number,
    size?: number
  ): void;
  submit(commandBuffers: readonly unknown[]): void;
  onSubmittedWorkDone?(): Promise<void>;
}

interface GpuDeviceLike {
  readonly queue: GpuQueueLike;
  createBuffer(descriptor: {
    readonly label?: string;
    readonly size: number;
    readonly usage: number;
    readonly mappedAtCreation?: boolean;
  }): GpuBufferLike;
  createShaderModule(descriptor: { readonly label?: string; readonly code: string }): unknown;
  createBindGroupLayout(descriptor: {
    readonly label?: string;
    readonly entries: readonly GpuBindGroupLayoutEntryLike[];
  }): unknown;
  createPipelineLayout(descriptor: {
    readonly label?: string;
    readonly bindGroupLayouts: readonly unknown[];
  }): unknown;
  createComputePipeline(descriptor: {
    readonly label?: string;
    readonly layout: unknown;
    readonly compute: { readonly module: unknown; readonly entryPoint: string };
  }): unknown;
  createBindGroup(descriptor: {
    readonly label?: string;
    readonly layout: unknown;
    readonly entries: readonly GpuBindGroupEntryLike[];
  }): unknown;
  createCommandEncoder(descriptor?: { readonly label?: string }): GpuCommandEncoderLike;
}

interface GpuBindGroupLayoutEntryLike {
  readonly binding: number;
  readonly visibility: number;
  readonly buffer: { readonly type: "read-only-storage" | "storage" };
}

interface GpuBindGroupEntryLike {
  readonly binding: number;
  readonly resource: { readonly buffer: GpuBufferLike };
}

interface GpuCommandEncoderLike {
  beginComputePass(descriptor?: { readonly label?: string }): GpuComputePassEncoderLike;
  copyBufferToBuffer(
    source: GpuBufferLike,
    sourceOffset: number,
    destination: GpuBufferLike,
    destinationOffset: number,
    size: number
  ): void;
  finish(): unknown;
}

interface GpuComputePassEncoderLike {
  setPipeline(pipeline: unknown): void;
  setBindGroup(index: number, bindGroup: unknown): void;
  dispatchWorkgroups(workgroupCountX: number): void;
  end(): void;
}

interface GpuBufferLike {
  mapAsync(mode: number, offset?: number, size?: number): Promise<void> | void;
  getMappedRange(offset?: number, size?: number): ArrayBuffer | ArrayBufferView;
  unmap(): void;
  destroy?(): void;
}

export class WebGpuMatrixBackend implements AsyncNeuralMatrixBackend<WebGpuMatrix> {
  private readonly unaryLayout: unknown;
  private readonly binaryLayout: unknown;
  private readonly scalePipeline: unknown;
  private readonly addPipeline: unknown;
  private readonly activationPipeline: unknown;

  private constructor(private readonly device: GpuDeviceLike) {
    this.unaryLayout = this.device.createBindGroupLayout({
      label: "neural-matrix-unary-layout",
      entries: [
        storageBinding(0, "read-only-storage"),
        storageBinding(1, "read-only-storage"),
        storageBinding(2, "storage"),
      ],
    });
    this.binaryLayout = this.device.createBindGroupLayout({
      label: "neural-matrix-binary-layout",
      entries: [
        storageBinding(0, "read-only-storage"),
        storageBinding(1, "read-only-storage"),
        storageBinding(2, "storage"),
      ],
    });
    this.scalePipeline = this.createPipeline("neural-matrix-scale", SCALE_SHADER, this.unaryLayout);
    this.addPipeline = this.createPipeline("neural-matrix-add", ADD_SHADER, this.binaryLayout);
    this.activationPipeline = this.createPipeline(
      "neural-matrix-activation",
      ACTIVATION_SHADER,
      this.unaryLayout
    );
  }

  static async create(
    gpu: WebGpuLike,
    options: WebGpuMatrixBackendOptions = {}
  ): Promise<WebGpuMatrixBackend> {
    const adapter = await gpu.requestAdapter(options);
    if (adapter === null) {
      throw new Error("WebGPU is available, but no adapter was returned");
    }
    return new WebGpuMatrixBackend(await adapter.requestDevice());
  }

  static async createFromNavigator(
    options: WebGpuMatrixBackendOptions = {}
  ): Promise<WebGpuMatrixBackend | null> {
    const gpu = getNavigatorGpu();
    if (gpu === undefined) {
      return null;
    }
    return WebGpuMatrixBackend.create(gpu, options);
  }

  static isNavigatorAvailable(): boolean {
    return getNavigatorGpu() !== undefined;
  }

  async fromRows(rows: readonly (readonly number[])[]): Promise<WebGpuMatrix> {
    const rowCount = rows.length;
    const colCount = rows[0]?.length ?? 0;
    const data = new Float32Array(rowCount * colCount);
    rows.forEach((row, rowIndex) => {
      if (row.length !== colCount) {
        throw new Error("All WebGPU matrix rows must have the same column count");
      }
      row.forEach((value, colIndex) => {
        data[rowIndex * colCount + colIndex] = value;
      });
    });
    return this.upload(data, rowCount, colCount, "neural-matrix-rows");
  }

  async toRows(matrix: WebGpuMatrix): Promise<number[][]> {
    const values = await this.download(matrix);
    return Array.from({ length: matrix.rows }, (_, rowIndex) => (
      Array.from(
        values.slice(rowIndex * matrix.cols, rowIndex * matrix.cols + matrix.cols)
      )
    ));
  }

  column(values: readonly number[]): WebGpuMatrix {
    return this.upload(new Float32Array(values), values.length, 1, "neural-matrix-column");
  }

  constant(value: number, rows: number, cols = 1): WebGpuMatrix {
    return this.upload(
      new Float32Array(rows * cols).fill(value),
      rows,
      cols,
      "neural-matrix-constant"
    );
  }

  add(left: WebGpuMatrix, right: WebGpuMatrix): WebGpuMatrix {
    assertSameShape(left, right);
    const output = this.createOutput(left.rows, left.cols, "neural-matrix-add-output");
    this.runBinary(this.addPipeline, left, right, output, "neural-matrix-add-pass");
    return output;
  }

  scale(matrix: WebGpuMatrix, scalar: number): WebGpuMatrix {
    const scalarBuffer = this.uploadParameter(new Float32Array([scalar]), "neural-matrix-scale-value");
    const output = this.createOutput(
      matrix.rows,
      matrix.cols,
      "neural-matrix-scale-output",
      [scalarBuffer]
    );
    this.runUnary(this.scalePipeline, matrix, scalarBuffer, output, "neural-matrix-scale-pass");
    return output;
  }

  activate(matrix: WebGpuMatrix, activation: string): WebGpuMatrix {
    const activationBuffer = this.uploadParameter(
      new Uint32Array([activationCode(activation)]),
      "neural-matrix-activation-code"
    );
    const output = this.createOutput(
      matrix.rows,
      matrix.cols,
      "neural-matrix-activation-output",
      [activationBuffer]
    );
    this.runUnary(
      this.activationPipeline,
      matrix,
      activationBuffer,
      output,
      "neural-matrix-activation-pass"
    );
    return output;
  }

  async toColumn(matrix: WebGpuMatrix): Promise<number[]> {
    if (matrix.cols !== 1) {
      throw new Error(`Expected a single-column WebGPU matrix, got ${matrix.cols} columns`);
    }
    return Array.from(await this.download(matrix));
  }

  dispose(matrix: WebGpuMatrix): void {
    matrix.buffer.destroy?.();
    matrix.scratch?.forEach((buffer) => buffer.destroy?.());
  }

  private createPipeline(label: string, code: string, layout: unknown): unknown {
    const module = this.device.createShaderModule({ label: `${label}-shader`, code });
    const pipelineLayout = this.device.createPipelineLayout({
      label: `${label}-pipeline-layout`,
      bindGroupLayouts: [layout],
    });
    return this.device.createComputePipeline({
      label,
      layout: pipelineLayout,
      compute: { module, entryPoint: "main" },
    });
  }

  private upload(
    data: Float32Array,
    rows: number,
    cols: number,
    label: string
  ): WebGpuMatrix {
    const byteLength = matrixByteLength(data.length);
    const buffer = this.device.createBuffer({
      label,
      size: byteLength,
      usage: GPU_BUFFER_USAGE.STORAGE | GPU_BUFFER_USAGE.COPY_SRC | GPU_BUFFER_USAGE.COPY_DST,
    });
    if (data.length > 0) {
      this.device.queue.writeBuffer(buffer, 0, data);
    }
    return { rows, cols, length: data.length, byteLength, buffer };
  }

  private uploadParameter(data: Float32Array | Uint32Array, label: string): GpuBufferLike {
    const buffer = this.device.createBuffer({
      label,
      size: matrixByteLength(data.length),
      usage: GPU_BUFFER_USAGE.STORAGE | GPU_BUFFER_USAGE.COPY_DST,
    });
    this.device.queue.writeBuffer(buffer, 0, data);
    return buffer;
  }

  private createOutput(
    rows: number,
    cols: number,
    label: string,
    scratch: readonly GpuBufferLike[] = []
  ): WebGpuMatrix {
    const length = rows * cols;
    const byteLength = matrixByteLength(length);
    const buffer = this.device.createBuffer({
      label,
      size: byteLength,
      usage: GPU_BUFFER_USAGE.STORAGE | GPU_BUFFER_USAGE.COPY_SRC | GPU_BUFFER_USAGE.COPY_DST,
    });
    return { rows, cols, length, byteLength, buffer, scratch };
  }

  private runBinary(
    pipeline: unknown,
    left: WebGpuMatrix,
    right: WebGpuMatrix,
    output: WebGpuMatrix,
    label: string
  ): void {
    const bindGroup = this.device.createBindGroup({
      label: `${label}-bind-group`,
      layout: this.binaryLayout,
      entries: [
        bufferBinding(0, left.buffer),
        bufferBinding(1, right.buffer),
        bufferBinding(2, output.buffer),
      ],
    });
    this.dispatch(pipeline, bindGroup, output.length, label);
  }

  private runUnary(
    pipeline: unknown,
    input: WebGpuMatrix,
    parameterBuffer: GpuBufferLike,
    output: WebGpuMatrix,
    label: string
  ): void {
    const bindGroup = this.device.createBindGroup({
      label: `${label}-bind-group`,
      layout: this.unaryLayout,
      entries: [
        bufferBinding(0, input.buffer),
        bufferBinding(1, parameterBuffer),
        bufferBinding(2, output.buffer),
      ],
    });
    this.dispatch(pipeline, bindGroup, output.length, label);
  }

  private dispatch(
    pipeline: unknown,
    bindGroup: unknown,
    length: number,
    label: string
  ): void {
    const encoder = this.device.createCommandEncoder({ label: `${label}-encoder` });
    const pass = encoder.beginComputePass({ label });
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.max(1, Math.ceil(length / WORKGROUP_SIZE)));
    pass.end();
    this.device.queue.submit([encoder.finish()]);
  }

  private async download(matrix: WebGpuMatrix): Promise<Float32Array> {
    const staging = this.device.createBuffer({
      label: "neural-matrix-readback",
      size: matrix.byteLength,
      usage: GPU_BUFFER_USAGE.MAP_READ | GPU_BUFFER_USAGE.COPY_DST,
    });
    const encoder = this.device.createCommandEncoder({ label: "neural-matrix-readback-encoder" });
    encoder.copyBufferToBuffer(matrix.buffer, 0, staging, 0, matrix.byteLength);
    this.device.queue.submit([encoder.finish()]);
    await this.device.queue.onSubmittedWorkDone?.();
    await staging.mapAsync(GPU_MAP_MODE.READ, 0, matrix.byteLength);
    const range = staging.getMappedRange(0, matrix.byteLength);
    const bytes = ArrayBuffer.isView(range)
      ? new Uint8Array(range.buffer, range.byteOffset, range.byteLength)
      : new Uint8Array(range);
    const copy = bytes.slice(0, matrix.length * Float32Array.BYTES_PER_ELEMENT);
    const values = new Float32Array(copy.buffer, copy.byteOffset, matrix.length);
    const result = new Float32Array(values);
    staging.unmap();
    staging.destroy?.();
    return result;
  }
}

export function getNavigatorGpu(): WebGpuLike | undefined {
  return (globalThis as typeof globalThis & {
    navigator?: { readonly gpu?: WebGpuLike };
  }).navigator?.gpu;
}

export async function createWebGpuMatrixBackend(
  options: WebGpuMatrixBackendOptions = {}
): Promise<WebGpuMatrixBackend | null> {
  return WebGpuMatrixBackend.createFromNavigator(options);
}

function storageBinding(
  binding: number,
  type: "read-only-storage" | "storage"
): GpuBindGroupLayoutEntryLike {
  return {
    binding,
    visibility: GPU_SHADER_STAGE.COMPUTE,
    buffer: { type },
  };
}

function bufferBinding(binding: number, buffer: GpuBufferLike): GpuBindGroupEntryLike {
  return {
    binding,
    resource: { buffer },
  };
}

function matrixByteLength(length: number): number {
  return Math.max(Float32Array.BYTES_PER_ELEMENT, length * Float32Array.BYTES_PER_ELEMENT);
}

function assertSameShape(left: WebGpuMatrix, right: WebGpuMatrix): void {
  if (left.rows !== right.rows || left.cols !== right.cols) {
    throw new Error(
      `WebGPU matrix shape mismatch: ${left.rows}x${left.cols} vs ${right.rows}x${right.cols}`
    );
  }
}

function activationCode(activation: string): number {
  switch (activation) {
    case "none":
    case "linear":
      return 0;
    case "relu":
      return 1;
    case "sigmoid":
      return 2;
    case "tanh":
      return 3;
    default:
      throw new Error(`Unsupported WebGPU activation: ${activation}`);
  }
}

const ADD_SHADER = `
@group(0) @binding(0) var<storage, read> left_values: array<f32>;
@group(0) @binding(1) var<storage, read> right_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(${WORKGROUP_SIZE})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = left_values[index] + right_values[index];
}
`;

const SCALE_SHADER = `
@group(0) @binding(0) var<storage, read> input_values: array<f32>;
@group(0) @binding(1) var<storage, read> scalar_values: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(${WORKGROUP_SIZE})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = input_values[index] * scalar_values[0];
}
`;

const ACTIVATION_SHADER = `
@group(0) @binding(0) var<storage, read> input_values: array<f32>;
@group(0) @binding(1) var<storage, read> activation_values: array<u32>;
@group(0) @binding(2) var<storage, read_write> output_values: array<f32>;

fn apply_activation(value: f32, activation: u32) -> f32 {
  switch activation {
    case 1u: {
      return max(value, 0.0);
    }
    case 2u: {
      return 1.0 / (1.0 + exp(-value));
    }
    case 3u: {
      let doubled = clamp(2.0 * value, -40.0, 40.0);
      let exponent = exp(doubled);
      return (exponent - 1.0) / (exponent + 1.0);
    }
    default: {
      return value;
    }
  }
}

@compute @workgroup_size(${WORKGROUP_SIZE})
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
  let index = global_id.x;
  if (index >= arrayLength(&output_values)) {
    return;
  }
  output_values[index] = apply_activation(input_values[index], activation_values[0]);
}
`;
