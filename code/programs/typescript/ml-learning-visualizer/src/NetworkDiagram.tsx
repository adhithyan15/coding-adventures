import { renderToSvgString } from "@coding-adventures/paint-vm-svg";
import {
  paintEllipse,
  paintLine,
  paintRect,
  paintScene,
  paintText,
  type PaintInstruction,
  type PaintScene,
} from "@coding-adventures/paint-instructions";
import type {
  HiddenLayerExample,
  HiddenLayerModelState,
  HiddenLayerStepResult,
} from "./hidden-layer-examples.js";
import type { ModelState, StepResult } from "./training.js";

const FONT_REF = "svg:ui-sans-serif@12";
const MUTED = "#5d6d68";
const PANEL = "#ffffff";
const LINE = "rgba(23, 32, 28, 0.16)";
const GREEN = "#237a57";
const BLUE = "#2563eb";
const RED = "#c2413b";
const GOLD = "#b7791f";

interface DiagramNode {
  readonly id: string;
  readonly label: string;
  readonly value: string;
  readonly x: number;
  readonly y: number;
  readonly tone: "input" | "hidden" | "output" | "bias";
}

export function LinearNetworkDiagram({
  model,
  lastStep,
  learningRate,
}: {
  model: ModelState;
  lastStep: StepResult | null;
  learningRate: number;
}) {
  return (
    <NetworkPanel
      title="Neural graph"
      summary="Linear graph VM view"
      svg={renderLinearNetworkSvg(model, lastStep, learningRate)}
    />
  );
}

export function HiddenNetworkDiagram({
  example,
  state,
  selectedInput,
  prediction,
  lastStep,
  learningRate,
}: {
  example: HiddenLayerExample;
  state: HiddenLayerModelState;
  selectedInput: readonly number[];
  prediction: number;
  lastStep: HiddenLayerStepResult | null;
  learningRate: number;
}) {
  return (
    <NetworkPanel
      title="Neural graph"
      summary="Hidden layer graph VM view"
      svg={renderHiddenNetworkSvg(
        example,
        state,
        selectedInput,
        prediction,
        lastStep,
        learningRate,
      )}
    />
  );
}

export function renderLinearNetworkSvg(
  model: ModelState,
  lastStep: StepResult | null,
  learningRate: number,
): string {
  const scene = makeLinearScene(model, lastStep, learningRate);
  return renderToSvgString(scene);
}

export function renderHiddenNetworkSvg(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  selectedInput: readonly number[],
  prediction: number,
  lastStep: HiddenLayerStepResult | null,
  learningRate: number,
): string {
  const scene = makeHiddenScene(
    example,
    state,
    selectedInput,
    prediction,
    lastStep,
    learningRate,
  );
  return renderToSvgString(scene);
}

function NetworkPanel({
  title,
  summary,
  svg,
}: {
  title: string;
  summary: string;
  svg: string;
}) {
  return (
    <section className="network-panel" aria-label={summary}>
      <div className="history__topline">
        <span>{title}</span>
        <strong>{summary}</strong>
      </div>
      <div className="network-svg" dangerouslySetInnerHTML={{ __html: svg }} />
    </section>
  );
}

function makeLinearScene(
  model: ModelState,
  lastStep: StepResult | null,
  learningRate: number,
): PaintScene {
  const width = 760;
  const height = 280;
  const input: DiagramNode = {
    id: "input",
    label: "x",
    value: "input",
    x: 120,
    y: 130,
    tone: "input",
  };
  const bias: DiagramNode = {
    id: "bias",
    label: "bias",
    value: fmt(model.bias),
    x: 120,
    y: 214,
    tone: "bias",
  };
  const sum: DiagramNode = {
    id: "sum",
    label: "sum",
    value: "x*w+b",
    x: 378,
    y: 130,
    tone: "hidden",
  };
  const output: DiagramNode = {
    id: "output",
    label: "pred",
    value: "y",
    x: 640,
    y: 130,
    tone: "output",
  };

  const dw = lastStep === null ? 0 : -learningRate * lastStep.gradientWeight;
  const db = lastStep === null ? 0 : -learningRate * lastStep.gradientBias;
  const instructions: PaintInstruction[] = [
    paintRect(16, 16, width - 32, height - 32, {
      fill: PANEL,
      stroke: LINE,
      stroke_width: 1,
      corner_radius: 8,
    }),
    ...edge(input, sum, model.weight, `w ${fmt(model.weight)}`),
    ...edge(bias, sum, model.bias, `b ${fmt(model.bias)}`),
    ...edge(sum, output, 1, "linear"),
    ...node(input),
    ...node(bias),
    ...node(sum),
    ...node(output),
    ...text(`epoch ${model.epoch}`, 32, 48, 13, MUTED),
    ...text(`dw ${signed(dw)}  db ${signed(db)}`, 32, 254, 13, MUTED),
    ...text("line width follows |weight|; green is positive, red is negative", 380, 254, 12, MUTED),
  ];

  return paintScene(width, height, "#ffffff", instructions, {
    id: "linear-neural-network-diagram",
  });
}

function makeHiddenScene(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  selectedInput: readonly number[],
  prediction: number,
  lastStep: HiddenLayerStepResult | null,
  learningRate: number,
): PaintScene {
  const width = 820;
  const height = 390;
  const inputYs = verticalPositions(example.inputLabels.length, 82, 232);
  const hiddenYs = verticalPositions(example.hiddenCount, 54, 286);
  const inputNodes = example.inputLabels.map((label, index): DiagramNode => ({
    id: `input-${index}`,
    label,
    value: fmt(selectedInput[index] ?? 0),
    x: 86,
    y: inputYs[index]!,
    tone: "input",
  }));
  const bias: DiagramNode = {
    id: "bias",
    label: "bias",
    value: "1",
    x: 86,
    y: 318,
    tone: "bias",
  };
  const hiddenNodes = Array.from({ length: example.hiddenCount }, (_, index): DiagramNode => ({
    id: `hidden-${index}`,
    label: `h${index + 1}`,
    value: fmt(sigmoid(rawHidden(state, selectedInput, index))),
    x: 392,
    y: hiddenYs[index]!,
    tone: "hidden",
  }));
  const output: DiagramNode = {
    id: "output",
    label: example.outputLabel,
    value: fmt(prediction),
    x: 720,
    y: 170,
    tone: "output",
  };

  const instructions: PaintInstruction[] = [
    paintRect(16, 16, width - 32, height - 32, {
      fill: PANEL,
      stroke: LINE,
      stroke_width: 1,
      corner_radius: 8,
    }),
    ...text(`epoch ${state.epoch}`, 32, 48, 13, MUTED),
    ...text("weights update on every step", 628, 48, 13, MUTED),
  ];

  for (const [inputIndex, inputNode] of inputNodes.entries()) {
    for (const [hiddenIndex, hiddenNode] of hiddenNodes.entries()) {
      const weight = state.parameters.inputToHiddenWeights[inputIndex]![hiddenIndex]!;
      instructions.push(...edge(inputNode, hiddenNode, weight, fmt(weight), 0.34));
    }
  }
  for (const [hiddenIndex, hiddenNode] of hiddenNodes.entries()) {
    const biasWeight = state.parameters.hiddenBiases[hiddenIndex]!;
    instructions.push(...edge(bias, hiddenNode, biasWeight, fmt(biasWeight), 0.26));
  }
  for (const [hiddenIndex, hiddenNode] of hiddenNodes.entries()) {
    const weight = state.parameters.hiddenToOutputWeights[hiddenIndex]![0]!;
    instructions.push(...edge(hiddenNode, output, weight, fmt(weight), 0.42));
  }
  instructions.push(
    ...edge(bias, output, state.parameters.outputBiases[0] ?? 0, fmt(state.parameters.outputBiases[0] ?? 0), 0.28),
    ...inputNodes.flatMap(node),
    ...node(bias),
    ...hiddenNodes.flatMap(node),
    ...node(output),
  );

  const inputShape = lastStep === null
    ? "input-hidden gradients waiting"
    : `input-hidden grad ${lastStep.step.inputToHiddenWeightGradients.length}x${lastStep.step.inputToHiddenWeightGradients[0]?.length ?? 0}`;
  const outputGrad = lastStep?.step.outputBiasGradients[0] ?? 0;
  instructions.push(
    ...text(inputShape, 32, 356, 12, MUTED),
    ...text(`output db ${signed(-learningRate * outputGrad)}`, 356, 356, 12, MUTED),
    ...text("line width = |weight|", 620, 356, 12, MUTED),
  );

  return paintScene(width, height, "#ffffff", instructions, {
    id: "hidden-neural-network-diagram",
  });
}

function edge(
  from: DiagramNode,
  to: DiagramNode,
  weight: number,
  label: string,
  labelOffset = 0.5,
): PaintInstruction[] {
  const midX = from.x + (to.x - from.x) * labelOffset;
  const midY = from.y + (to.y - from.y) * labelOffset;
  const color = weight >= 0 ? GREEN : RED;
  const width = Math.min(7, 1.4 + Math.abs(weight) * 0.75);
  return [
    paintLine(from.x, from.y, to.x, to.y, color, {
      stroke_width: width,
      stroke_cap: "round",
    }),
    paintRect(midX - 28, midY - 14, 56, 20, {
      fill: "rgba(255, 255, 255, 0.86)",
      stroke: "rgba(23, 32, 28, 0.08)",
      stroke_width: 1,
      corner_radius: 5,
    }),
    ...centerText(label, midX, midY + 4, 10, color),
  ];
}

function node(nodeData: DiagramNode): PaintInstruction[] {
  const fill = nodeFill(nodeData.tone);
  return [
    paintEllipse(nodeData.x, nodeData.y, 28, 28, {
      fill,
      stroke: "#ffffff",
      stroke_width: 3,
    }),
    paintEllipse(nodeData.x, nodeData.y, 31, 31, {
      stroke: LINE,
      stroke_width: 1,
    }),
    ...centerText(nodeData.label, nodeData.x, nodeData.y - 3, 11, "#ffffff"),
    ...centerText(nodeData.value, nodeData.x, nodeData.y + 12, 10, "#ffffff"),
  ];
}

function text(
  value: string,
  x: number,
  y: number,
  fontSize: number,
  fill: string,
  align: "start" | "center" | "end" = "start",
): PaintInstruction[] {
  return [paintText(x, y, value, FONT_REF, fontSize, fill, { text_align: align })];
}

function centerText(
  value: string,
  x: number,
  y: number,
  fontSize: number,
  fill: string,
): PaintInstruction[] {
  return text(value, x, y, fontSize, fill, "center");
}

function verticalPositions(count: number, top: number, bottom: number): number[] {
  if (count <= 1) {
    return [(top + bottom) / 2];
  }
  const span = bottom - top;
  return Array.from({ length: count }, (_, index) => top + (span * index) / (count - 1));
}

function rawHidden(
  state: HiddenLayerModelState,
  input: readonly number[],
  hiddenIndex: number,
): number {
  let raw = state.parameters.hiddenBiases[hiddenIndex] ?? 0;
  for (const [inputIndex, value] of input.entries()) {
    raw += value * (state.parameters.inputToHiddenWeights[inputIndex]?.[hiddenIndex] ?? 0);
  }
  return raw;
}

function sigmoid(value: number): number {
  if (value >= 0) {
    const z = Math.exp(-value);
    return 1 / (1 + z);
  }
  const z = Math.exp(value);
  return z / (1 + z);
}

function nodeFill(tone: DiagramNode["tone"]): string {
  switch (tone) {
    case "input":
      return BLUE;
    case "hidden":
      return GREEN;
    case "output":
      return GOLD;
    case "bias":
      return "#6d5bd0";
  }
}

function fmt(value: number): string {
  if (!Number.isFinite(value)) {
    return "0";
  }
  if (Math.abs(value) >= 10) {
    return value.toFixed(1);
  }
  return value.toFixed(2);
}

function signed(value: number): string {
  return `${value >= 0 ? "+" : ""}${fmt(value)}`;
}
