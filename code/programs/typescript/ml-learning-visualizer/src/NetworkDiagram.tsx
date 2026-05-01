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
  HiddenLayerExampleRow,
  HiddenLayerModelState,
  HiddenLayerStepResult,
} from "./hidden-layer-examples.js";
import type { LossKind, ModelState, StepResult, TrainingPoint } from "./training.js";

const FONT_REF = "svg:ui-sans-serif@12";
const MUTED = "#5d6d68";
const PANEL = "#ffffff";
const LINE = "rgba(23, 32, 28, 0.16)";
const GREEN = "#237a57";
const BLUE = "#2563eb";
const RED = "#c2413b";
const GOLD = "#b7791f";
const VIOLET = "#6d5bd0";

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
  lossKind,
  samplePoint,
  pointCount,
}: {
  model: ModelState;
  lastStep: StepResult | null;
  learningRate: number;
  lossKind: LossKind;
  samplePoint: TrainingPoint;
  pointCount: number;
}) {
  return (
    <NetworkPanel
      title="Learning flow"
      summary="Forward pass and gradient descent"
      svg={renderLinearNetworkSvg(
        model,
        lastStep,
        learningRate,
        lossKind,
        samplePoint,
        pointCount,
      )}
    />
  );
}

export function HiddenNetworkDiagram({
  example,
  state,
  selectedRow,
  selectedIndex,
  prediction,
  lastStep,
  learningRate,
}: {
  example: HiddenLayerExample;
  state: HiddenLayerModelState;
  selectedRow: HiddenLayerExampleRow;
  selectedIndex: number;
  prediction: number;
  lastStep: HiddenLayerStepResult | null;
  learningRate: number;
}) {
  return (
    <NetworkPanel
      title="Neural graph"
      summary="Hidden layer learning flow"
      svg={renderHiddenNetworkSvg(
        example,
        state,
        selectedRow,
        selectedIndex,
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
  lossKind: LossKind = "mse",
  samplePoint: TrainingPoint = { x: 0, y: 0 },
  pointCount = 1,
): string {
  const scene = makeLinearScene(
    model,
    lastStep,
    learningRate,
    lossKind,
    samplePoint,
    pointCount,
  );
  return renderToSvgString(scene);
}

export function renderHiddenNetworkSvg(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  selectedRow: HiddenLayerExampleRow,
  selectedIndex: number,
  prediction: number,
  lastStep: HiddenLayerStepResult | null,
  learningRate: number,
): string {
  const scene = makeHiddenScene(
    example,
    state,
    selectedRow,
    selectedIndex,
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
  lossKind: LossKind,
  samplePoint: TrainingPoint,
  pointCount: number,
): PaintScene {
  const width = 920;
  const height = 520;
  const runState = lastStep?.previousState ?? model;
  const prediction = samplePoint.x * runState.weight + runState.bias;
  const error = prediction - samplePoint.y;
  const sampleLoss = lossKind === "mse" ? error * error : Math.abs(error);
  const input: DiagramNode = {
    id: "input",
    label: "x",
    value: fmt(samplePoint.x),
    x: 100,
    y: 150,
    tone: "input",
  };
  const bias: DiagramNode = {
    id: "bias",
    label: "bias",
    value: fmt(runState.bias),
    x: 100,
    y: 232,
    tone: "bias",
  };
  const sum: DiagramNode = {
    id: "sum",
    label: "sum",
    value: "x*w+b",
    x: 318,
    y: 150,
    tone: "hidden",
  };
  const output: DiagramNode = {
    id: "output",
    label: "pred",
    value: fmt(prediction),
    x: 540,
    y: 150,
    tone: "output",
  };
  const target: DiagramNode = {
    id: "target",
    label: "target",
    value: fmt(samplePoint.y),
    x: 540,
    y: 232,
    tone: "bias",
  };
  const lossNode: DiagramNode = {
    id: "loss",
    label: lossKind,
    value: fmt(sampleLoss),
    x: 760,
    y: 190,
    tone: "output",
  };

  const dw = lastStep === null ? 0 : -learningRate * lastStep.gradientWeight;
  const db = lastStep === null ? 0 : -learningRate * lastStep.gradientBias;
  const gradientText = lastStep === null
    ? "waiting for first step"
    : `dL/dw ${fmt(lastStep.gradientWeight)}  dL/db ${fmt(lastStep.gradientBias)}`;
  const updateText = lastStep === null
    ? "run Step to update weights"
    : `w ${fmt(lastStep.previousState.weight)} -> ${fmt(model.weight)}`;
  const biasUpdateText = lastStep === null
    ? `lr ${fmt(learningRate)}`
    : `b ${fmt(lastStep.previousState.bias)} -> ${fmt(model.bias)}`;
  const instructions: PaintInstruction[] = [
    paintRect(16, 16, width - 32, height - 32, {
      fill: PANEL,
      stroke: LINE,
      stroke_width: 1,
      corner_radius: 8,
    }),
    ...sectionLabel("1 forward pass", 36, 48),
    ...edge(input, sum, runState.weight, `w ${fmt(runState.weight)}`),
    ...edge(bias, sum, runState.bias, `b ${fmt(runState.bias)}`),
    ...edge(sum, output, 1, "linear"),
    ...edge(output, lossNode, error, `err ${signed(error)}`, 0.56),
    ...edge(target, lossNode, -1, "truth", 0.56),
    ...node(input),
    ...node(bias),
    ...node(sum),
    ...node(output),
    ...node(target),
    ...node(lossNode),
    ...flowCard(36, 314, 152, "input batch", [
      `${pointCount} rows`,
      `sample x ${fmt(samplePoint.x)}`,
      `target ${fmt(samplePoint.y)}`,
    ], BLUE),
    ...flowArrow(194, 360, 242, 360, BLUE, "feed"),
    ...flowCard(252, 314, 158, "prediction", [
      `yhat=x*w+b`,
      `yhat ${fmt(prediction)}`,
      "activation linear",
    ], GREEN),
    ...flowArrow(416, 360, 464, 360, BLUE, "compare"),
    ...flowCard(474, 314, 162, "error + loss", [
      `error ${signed(error)}`,
      `${lossKind.toUpperCase()} ${fmt(sampleLoss)}`,
      `batch loss ${fmt(lastStep?.previousLoss ?? sampleLoss)}`,
    ], RED),
    ...flowArrow(642, 360, 690, 360, RED, "differentiate"),
    ...flowCard(700, 314, 184, "gradient descent", [
      gradientText,
      `dw step ${signed(dw)}`,
      `db step ${signed(db)}`,
    ], VIOLET),
    ...flowArrow(792, 308, 792, 266, RED, "backprop"),
    ...flowArrow(700, 266, 332, 210, RED, "apply update"),
    ...text(updateText, 36, 488, 13, MUTED),
    ...text(biasUpdateText, 252, 488, 13, MUTED),
    ...text("parameter update: new = old - learningRate * gradient", 474, 488, 13, MUTED),
    ...text(`epoch ${model.epoch}`, 36, 72, 13, MUTED),
    ...text("line width follows |weight|; green is positive, red is negative", 476, 72, 12, MUTED),
  ];

  return paintScene(width, height, "#ffffff", instructions, {
    id: "linear-neural-network-diagram",
  });
}

function makeHiddenScene(
  example: HiddenLayerExample,
  state: HiddenLayerModelState,
  selectedRow: HiddenLayerExampleRow,
  selectedIndex: number,
  prediction: number,
  lastStep: HiddenLayerStepResult | null,
  learningRate: number,
): PaintScene {
  const width = 920;
  const height = 560;
  const selectedInput = selectedRow.input;
  const selectedError = prediction - selectedRow.target;
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
    x: 708,
    y: 170,
    tone: "output",
  };
  const target: DiagramNode = {
    id: "target",
    label: "target",
    value: fmt(selectedRow.target),
    x: 708,
    y: 252,
    tone: "bias",
  };
  const lossNode: DiagramNode = {
    id: "loss",
    label: "mse",
    value: fmt(selectedError * selectedError),
    x: 834,
    y: 212,
    tone: "output",
  };

  const instructions: PaintInstruction[] = [
    paintRect(16, 16, width - 32, height - 32, {
      fill: PANEL,
      stroke: LINE,
      stroke_width: 1,
      corner_radius: 8,
    }),
    ...sectionLabel("1 selected forward pass", 32, 48),
    ...text(`epoch ${state.epoch}`, 32, 70, 13, MUTED),
    ...text("weights update on every batch step", 630, 48, 13, MUTED),
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
    ...edge(output, lossNode, selectedError, `err ${signed(selectedError)}`, 0.62),
    ...edge(target, lossNode, -1, "truth", 0.62),
    ...inputNodes.flatMap(node),
    ...node(bias),
    ...hiddenNodes.flatMap(node),
    ...node(output),
    ...node(target),
    ...node(lossNode),
  );

  const inputShape = lastStep === null
    ? "input-hidden gradients waiting"
    : `input-hidden grad ${lastStep.step.inputToHiddenWeightGradients.length}x${lastStep.step.inputToHiddenWeightGradients[0]?.length ?? 0}`;
  const outputGrad = lastStep?.step.outputBiasGradients[0] ?? 0;
  const outputDelta = lastStep?.step.outputDeltas[selectedIndex]?.[0] ?? 0;
  const hiddenDeltaRow = lastStep?.step.hiddenDeltas[selectedIndex] ?? [];
  const hiddenDelta = hiddenDeltaRow.length === 0
    ? 0
    : hiddenDeltaRow.reduce((max, value) => Math.max(max, Math.abs(value)), 0);
  const hiddenUpdate = lastStep === null
    ? "waiting for first step"
    : `max hidden delta ${fmt(hiddenDelta)}`;
  instructions.push(
    ...sectionLabel("2 loss, deltas, and gradient descent", 32, 352),
    ...flowCard(32, 382, 158, "input row", [
      selectedRow.label,
      `inputs ${selectedInput.map((value) => fmt(value)).join(", ")}`,
      `target ${fmt(selectedRow.target)}`,
    ], BLUE),
    ...flowArrow(196, 428, 244, 428, BLUE, "forward"),
    ...flowCard(254, 382, 162, "prediction", [
      `hidden[${example.hiddenCount}]`,
      `${example.outputLabel} ${fmt(prediction)}`,
      `error ${signed(selectedError)}`,
    ], GREEN),
    ...flowArrow(422, 428, 470, 428, RED, "loss"),
    ...flowCard(480, 382, 158, "mse + deltas", [
      `row mse ${fmt(selectedError * selectedError)}`,
      `output delta ${fmt(outputDelta)}`,
      hiddenUpdate,
    ], RED),
    ...flowArrow(644, 428, 692, 428, RED, "gradients"),
    ...flowCard(702, 382, 186, "parameter update", [
      inputShape,
      `hidden-output ${gradientShape(lastStep?.step.hiddenToOutputWeightGradients)}`,
      `db out ${signed(-learningRate * outputGrad)}`,
    ], VIOLET),
    ...flowArrow(800, 376, 800, 330, RED, "backprop"),
    ...flowArrow(708, 330, 430, 276, RED, "update matrices"),
    ...text("new weights = old weights - learningRate * gradient", 32, 536, 13, MUTED),
    ...text("line width = |weight|", 630, 536, 12, MUTED),
  );

  return paintScene(width, height, "#ffffff", instructions, {
    id: "hidden-neural-network-diagram",
  });
}

function sectionLabel(label: string, x: number, y: number): PaintInstruction[] {
  return [
    paintRect(x - 8, y - 17, Math.max(126, label.length * 8), 24, {
      fill: "rgba(35, 122, 87, 0.1)",
      stroke: "rgba(35, 122, 87, 0.18)",
      stroke_width: 1,
      corner_radius: 6,
    }),
    ...text(label, x, y, 13, GREEN),
  ];
}

function flowCard(
  x: number,
  y: number,
  width: number,
  title: string,
  lines: readonly string[],
  color: string,
): PaintInstruction[] {
  const height = 126;
  const instructions: PaintInstruction[] = [
    paintRect(x, y, width, height, {
      fill: "#ffffff",
      stroke: "rgba(23, 32, 28, 0.12)",
      stroke_width: 1,
      corner_radius: 8,
    }),
    paintRect(x, y, width, 30, {
      fill: "rgba(247, 248, 243, 0.95)",
      stroke: "rgba(23, 32, 28, 0.08)",
      stroke_width: 1,
      corner_radius: 8,
    }),
    ...text(title, x + 12, y + 21, 12, color),
  ];
  for (const [index, line] of lines.entries()) {
    instructions.push(...text(compactLine(line, width), x + 12, y + 54 + index * 22, 11, MUTED));
  }
  return instructions;
}

function flowArrow(
  x1: number,
  y1: number,
  x2: number,
  y2: number,
  color: string,
  label: string,
): PaintInstruction[] {
  const angle = Math.atan2(y2 - y1, x2 - x1);
  const head = 9;
  const left = angle + Math.PI * 0.82;
  const right = angle - Math.PI * 0.82;
  const labelX = x1 + (x2 - x1) * 0.5;
  const labelY = y1 + (y2 - y1) * 0.5 - 7;

  return [
    paintLine(x1, y1, x2, y2, color, {
      stroke_width: 2,
      stroke_cap: "round",
    }),
    paintLine(x2, y2, x2 + Math.cos(left) * head, y2 + Math.sin(left) * head, color, {
      stroke_width: 2,
      stroke_cap: "round",
    }),
    paintLine(x2, y2, x2 + Math.cos(right) * head, y2 + Math.sin(right) * head, color, {
      stroke_width: 2,
      stroke_cap: "round",
    }),
    ...centerText(label, labelX, labelY, 10, color),
  ];
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

function gradientShape(rows: readonly (readonly number[])[] | undefined): string {
  if (rows === undefined || rows.length === 0) {
    return "0x0";
  }
  return `${rows.length}x${rows[0]?.length ?? 0}`;
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

function compactLine(value: string, width: number): string {
  const maxChars = Math.max(10, Math.floor(width / 7.2));
  if (value.length <= maxChars) {
    return value;
  }
  return `${value.slice(0, maxChars - 3)}...`;
}
