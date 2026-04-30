import { Matrix } from "matrix";

import {
  applyNeuralActivation,
  type NeuralBytecodeInstruction,
  type NeuralBytecodeModule,
} from "./neural-graph-vm.js";

export type NeuralMatrixPlanOpcode =
  | "LOAD_INPUT_MATRIX"
  | "LOAD_CONST_MATRIX"
  | "WEIGHTED_SUM_MATRIX"
  | "ACTIVATE_MATRIX"
  | "STORE_OUTPUT_MATRIX";

export interface NeuralMatrixPlanTerm {
  readonly sourceValue: string;
  readonly sourceNode?: string;
  readonly edgeId: string;
  readonly weight: number;
}

export interface NeuralMatrixPlanInstruction {
  readonly op: NeuralMatrixPlanOpcode;
  readonly dst?: string;
  readonly inputName?: string;
  readonly outputName?: string;
  readonly value?: number;
  readonly terms?: readonly NeuralMatrixPlanTerm[];
  readonly input?: string;
  readonly activation?: string;
  readonly sourceNode?: string;
  readonly sourceEdge?: string;
  readonly sourceInstructionIndexes: readonly number[];
}

export interface NeuralMatrixPlan {
  readonly magic: "CANM";
  readonly version: 0;
  readonly sourceBytecodeVersion: 0;
  readonly instructions: readonly NeuralMatrixPlanInstruction[];
}

export type NeuralMatrixInputValue = number | readonly number[];
export type NeuralMatrixInputs = Record<string, NeuralMatrixInputValue>;

export interface NeuralMatrixForwardResult {
  readonly outputs: Record<string, number[]>;
  readonly values: Record<string, number[]>;
}

export interface MatrixBackend<M> {
  fromRows(rows: readonly (readonly number[])[]): M;
  toRows(matrix: M): number[][];
  column(values: readonly number[]): M;
  constant(value: number, rows: number, cols?: number): M;
  add(left: M, right: M): M;
  scale(matrix: M, scalar: number): M;
  dot(left: M, right: M): M;
  map(matrix: M, fn: (value: number) => number): M;
  toColumn(matrix: M): number[];
}

export class TypeScriptMatrixBackend implements MatrixBackend<Matrix> {
  fromRows(rows: readonly (readonly number[])[]): Matrix {
    return new Matrix(cloneRows(rows));
  }

  toRows(matrix: Matrix): number[][] {
    return cloneRows(matrix.data);
  }

  column(values: readonly number[]): Matrix {
    return new Matrix(values.map((value) => [value]));
  }

  constant(value: number, rows: number, cols = 1): Matrix {
    return new Matrix(
      Array.from({ length: rows }, () => Array(cols).fill(value))
    );
  }

  add(left: Matrix, right: Matrix): Matrix {
    return left.add(right);
  }

  scale(matrix: Matrix, scalar: number): Matrix {
    return matrix.scale(scalar);
  }

  dot(left: Matrix, right: Matrix): Matrix {
    return left.dot(right);
  }

  map(matrix: Matrix, fn: (value: number) => number): Matrix {
    return matrix.map(fn);
  }

  toColumn(matrix: Matrix): number[] {
    if (matrix.cols !== 1) {
      throw new Error(
        `Expected a single-column matrix, got ${matrix.cols} columns`
      );
    }
    return matrix.data.map((row) => row[0]);
  }
}

export function compileBytecodeToMatrixPlan(
  module: NeuralBytecodeModule
): NeuralMatrixPlan {
  const forward = module.functions.find((fn) => fn.kind === "forward");
  if (forward === undefined) {
    throw new Error("Neural bytecode module has no forward function");
  }

  const edgeWeights = new Map(
    module.graph.edges.map((edge) => [edge.id, edge.weight])
  );
  const valueSources = new Map<string, ValueSource>();
  const edgeWeightValues = new Map<string, EdgeWeightValue>();
  const termValues = new Map<string, NeuralMatrixPlanTerm>();
  const instructions: NeuralMatrixPlanInstruction[] = [];

  for (const [index, instruction] of forward.instructions.entries()) {
    switch (instruction.op) {
      case "LOAD_INPUT": {
        const dst = requireDst(instruction);
        valueSources.set(dst, {
          valueId: dst,
          sourceNode: instruction.sourceNode,
        });
        instructions.push({
          op: "LOAD_INPUT_MATRIX",
          dst,
          inputName: instruction.inputName,
          sourceNode: instruction.sourceNode,
          sourceInstructionIndexes: [index],
        });
        break;
      }
      case "LOAD_CONST": {
        const dst = requireDst(instruction);
        valueSources.set(dst, {
          valueId: dst,
          sourceNode: instruction.sourceNode,
        });
        instructions.push({
          op: "LOAD_CONST_MATRIX",
          dst,
          value: instruction.value ?? 0,
          sourceNode: instruction.sourceNode,
          sourceInstructionIndexes: [index],
        });
        break;
      }
      case "LOAD_EDGE_WEIGHT": {
        const dst = requireDst(instruction);
        const edgeId = requireEdgeId(instruction);
        edgeWeightValues.set(dst, {
          valueId: dst,
          edgeId,
          weight: edgeWeights.get(edgeId) ?? 1,
        });
        break;
      }
      case "MUL": {
        const dst = requireDst(instruction);
        const term = lowerWeightedTerm(
          instruction,
          valueSources,
          edgeWeightValues
        );
        termValues.set(dst, term);
        break;
      }
      case "ADD": {
        const dst = requireDst(instruction);
        const terms = (instruction.inputs ?? []).map((valueId) => {
          const term = termValues.get(valueId);
          if (term === undefined) {
            throw new Error(`Cannot lower ADD input ${valueId} to a matrix term`);
          }
          return term;
        });
        valueSources.set(dst, {
          valueId: dst,
          sourceNode: instruction.sourceNode,
        });
        instructions.push({
          op: "WEIGHTED_SUM_MATRIX",
          dst,
          terms,
          sourceNode: instruction.sourceNode,
          sourceInstructionIndexes: [index],
        });
        break;
      }
      case "ACTIVATE": {
        const dst = requireDst(instruction);
        requireValueSource(instruction.input, valueSources);
        valueSources.set(dst, {
          valueId: dst,
          sourceNode: instruction.sourceNode,
        });
        instructions.push({
          op: "ACTIVATE_MATRIX",
          dst,
          input: instruction.input,
          activation: instruction.activation ?? "relu",
          sourceNode: instruction.sourceNode,
          sourceInstructionIndexes: [index],
        });
        break;
      }
      case "STORE_OUTPUT": {
        requireValueSource(instruction.input, valueSources);
        instructions.push({
          op: "STORE_OUTPUT_MATRIX",
          outputName: instruction.outputName ?? "output",
          input: instruction.input,
          sourceNode: instruction.sourceNode,
          sourceInstructionIndexes: [index],
        });
        break;
      }
    }
  }

  return {
    magic: "CANM",
    version: 0,
    sourceBytecodeVersion: module.version,
    instructions,
  };
}

export function runNeuralMatrixForward(
  plan: NeuralMatrixPlan,
  inputs: NeuralMatrixInputs
): NeuralMatrixForwardResult;
export function runNeuralMatrixForward<M>(
  plan: NeuralMatrixPlan,
  inputs: NeuralMatrixInputs,
  backend: MatrixBackend<M>
): NeuralMatrixForwardResult;
export function runNeuralMatrixForward<M>(
  plan: NeuralMatrixPlan,
  inputs: NeuralMatrixInputs,
  backend: MatrixBackend<M> = new TypeScriptMatrixBackend() as MatrixBackend<M>
): NeuralMatrixForwardResult {
  const batchSize = inferBatchSize(inputs);
  const values = new Map<string, M>();
  const outputs: Record<string, number[]> = {};

  for (const instruction of plan.instructions) {
    switch (instruction.op) {
      case "LOAD_INPUT_MATRIX": {
        const dst = requirePlanDst(instruction);
        const inputName = instruction.inputName ?? instruction.sourceNode ?? dst;
        values.set(
          dst,
          backend.column(readInputColumn(inputs, inputName, batchSize))
        );
        break;
      }
      case "LOAD_CONST_MATRIX": {
        const dst = requirePlanDst(instruction);
        values.set(dst, backend.constant(instruction.value ?? 0, batchSize));
        break;
      }
      case "WEIGHTED_SUM_MATRIX": {
        const dst = requirePlanDst(instruction);
        const terms = instruction.terms ?? [];
        let result: M | undefined;
        for (const term of terms) {
          const weighted = backend.scale(
            readMatrixValue(values, term.sourceValue),
            term.weight
          );
          result = result === undefined ? weighted : backend.add(result, weighted);
        }
        values.set(dst, result ?? backend.constant(0, batchSize));
        break;
      }
      case "ACTIVATE_MATRIX": {
        const dst = requirePlanDst(instruction);
        values.set(
          dst,
          backend.map(
            readMatrixValue(values, instruction.input),
            (value) => applyNeuralActivation(value, instruction.activation ?? "relu")
          )
        );
        break;
      }
      case "STORE_OUTPUT_MATRIX": {
        const outputName = instruction.outputName ?? "output";
        outputs[outputName] = backend.toColumn(
          readMatrixValue(values, instruction.input)
        );
        break;
      }
    }
  }

  return {
    outputs,
    values: Object.fromEntries(
      [...values.entries()].map(([valueId, matrix]) => [
        valueId,
        backend.toColumn(matrix),
      ])
    ),
  };
}

export function runNeuralMatrixForwardScalars(
  plan: NeuralMatrixPlan,
  inputs: Record<string, number>,
  backend?: MatrixBackend<unknown>
): Record<string, number> {
  const result = backend === undefined
    ? runNeuralMatrixForward(plan, inputs)
    : runNeuralMatrixForward(plan, inputs, backend);

  return Object.fromEntries(
    Object.entries(result.outputs).map(([outputName, values]) => [
      outputName,
      values[0] ?? 0,
    ])
  );
}

interface ValueSource {
  readonly valueId: string;
  readonly sourceNode?: string;
}

interface EdgeWeightValue {
  readonly valueId: string;
  readonly edgeId: string;
  readonly weight: number;
}

function lowerWeightedTerm(
  instruction: NeuralBytecodeInstruction,
  valueSources: ReadonlyMap<string, ValueSource>,
  edgeWeightValues: ReadonlyMap<string, EdgeWeightValue>
): NeuralMatrixPlanTerm {
  const leftValue = valueSources.get(instruction.left ?? "");
  const rightValue = valueSources.get(instruction.right ?? "");
  const leftWeight = edgeWeightValues.get(instruction.left ?? "");
  const rightWeight = edgeWeightValues.get(instruction.right ?? "");

  if (leftValue !== undefined && rightWeight !== undefined) {
    return {
      sourceValue: leftValue.valueId,
      sourceNode: leftValue.sourceNode,
      edgeId: rightWeight.edgeId,
      weight: rightWeight.weight,
    };
  }
  if (rightValue !== undefined && leftWeight !== undefined) {
    return {
      sourceValue: rightValue.valueId,
      sourceNode: rightValue.sourceNode,
      edgeId: leftWeight.edgeId,
      weight: leftWeight.weight,
    };
  }

  throw new Error(
    `Cannot lower MUL ${instruction.dst ?? "<unknown>"} to a weighted matrix term`
  );
}

function inferBatchSize(inputs: NeuralMatrixInputs): number {
  let batchSize = 1;
  for (const value of Object.values(inputs)) {
    if (Array.isArray(value)) {
      if (value.length === 0) {
        throw new Error("Batched inputs must contain at least one value");
      }
      if (batchSize !== 1 && value.length !== batchSize) {
        throw new Error("All batched inputs must have the same length");
      }
      batchSize = value.length;
    }
  }
  return batchSize;
}

function readInputColumn(
  inputs: NeuralMatrixInputs,
  inputName: string,
  batchSize: number
): number[] {
  if (!(inputName in inputs)) {
    throw new Error(`Missing input: ${inputName}`);
  }
  const value = inputs[inputName];
  if (Array.isArray(value)) {
    if (value.length !== batchSize) {
      throw new Error("All batched inputs must have the same length");
    }
    return [...value];
  }
  return Array(batchSize).fill(value);
}

function requireDst(instruction: NeuralBytecodeInstruction): string {
  if (instruction.dst === undefined) {
    throw new Error(`Instruction ${instruction.op} is missing dst`);
  }
  return instruction.dst;
}

function requireEdgeId(instruction: NeuralBytecodeInstruction): string {
  if (instruction.edgeId === undefined) {
    throw new Error("LOAD_EDGE_WEIGHT is missing edgeId");
  }
  return instruction.edgeId;
}

function requireValueSource(
  valueId: string | undefined,
  values: ReadonlyMap<string, ValueSource>
): void {
  if (valueId === undefined || !values.has(valueId)) {
    throw new Error(`Cannot lower missing value: ${valueId ?? "<undefined>"}`);
  }
}

function requirePlanDst(instruction: NeuralMatrixPlanInstruction): string {
  if (instruction.dst === undefined) {
    throw new Error(`Matrix plan instruction ${instruction.op} is missing dst`);
  }
  return instruction.dst;
}

function readMatrixValue<M>(
  values: ReadonlyMap<string, M>,
  valueId: string | undefined
): M {
  if (valueId === undefined || !values.has(valueId)) {
    throw new Error(`Missing matrix value: ${valueId ?? "<undefined>"}`);
  }
  return values.get(valueId)!;
}

function cloneRows(rows: readonly (readonly number[])[]): number[][] {
  return rows.map((row) => [...row]);
}
