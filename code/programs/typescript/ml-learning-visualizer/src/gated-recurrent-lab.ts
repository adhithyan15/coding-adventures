export type GatedModel = "gru" | "lstm";
export type GruGate = "reset" | "update";
export type LstmGate = "forget" | "input" | "output";
export type GatedCellGate = GruGate | LstmGate;

export interface GateTrace {
  preactivation: number;
  value: number;
}

export interface GruGateTrace {
  resetGate: GateTrace;
  updateGate: GateTrace;
  candidate: {
    inputProduct: number;
    resetState: number;
    recurrentProduct: number;
    bias: number;
    preactivation: number;
    value: number;
  };
  retainedState: number;
  candidateWrite: number;
  hiddenState: number;
}

export interface LstmGateTrace {
  forgetGate: GateTrace;
  inputGate: GateTrace;
  outputGate: GateTrace;
  candidate: GateTrace;
  retainedCell: number;
  candidateWrite: number;
  cellState: number;
  exposedCell: number;
  hiddenState: number;
}

export interface GatedRecurrentTrace {
  input: number;
  previousHidden: number;
  previousCell: number;
  gru: GruGateTrace;
  lstm: LstmGateTrace;
}

export interface GateCounterfactual {
  model: GatedModel;
  gate: GatedCellGate;
  gateValue: number;
  candidate: number;
  cellState: number | null;
  hiddenState: number;
}

export const DEFAULT_GATED_INPUT = 1;
export const DEFAULT_GATED_PREVIOUS_HIDDEN = 0.8;
export const DEFAULT_GATED_PREVIOUS_CELL = 0.8;
export const DEFAULT_GATE_VALUES = {
  reset: 0.5,
  update: 0.25,
  forget: 0.5,
  input: 0.25,
  output: 0.75,
} as const;

const LOG_THREE = Math.log(3);
const CANDIDATE_PREACTIVATION = Math.atanh(0.6);
const GRU_CANDIDATE_BIAS = CANDIDATE_PREACTIVATION - 0.4;

function sigmoid(value: number): number {
  if (value >= 0) {
    const factor = Math.exp(-value);
    return 1 / (1 + factor);
  }
  const factor = Math.exp(value);
  return factor / (1 + factor);
}

function gate(preactivation: number): GateTrace {
  return { preactivation, value: sigmoid(preactivation) };
}

export function traceGatedRecurrent(
  input = DEFAULT_GATED_INPUT,
  previousHidden = DEFAULT_GATED_PREVIOUS_HIDDEN,
  previousCell = DEFAULT_GATED_PREVIOUS_CELL,
): GatedRecurrentTrace {
  if (![input, previousHidden, previousCell].every(Number.isFinite)) {
    throw new Error("NN11 V1 needs finite scalar input and recurrent states.");
  }

  const resetGate = gate(0);
  const updateGate = gate(-LOG_THREE);
  const resetState = resetGate.value * previousHidden;
  const inputProduct = 0 * input;
  const recurrentProduct = resetState;
  const gruCandidatePreactivation = inputProduct
    + recurrentProduct
    + GRU_CANDIDATE_BIAS;
  const gruCandidateValue = Math.tanh(gruCandidatePreactivation);
  const retainedState = (1 - updateGate.value) * previousHidden;
  const gruCandidateWrite = updateGate.value * gruCandidateValue;

  const forgetGate = gate(0);
  const inputGate = gate(-LOG_THREE);
  const outputGate = gate(LOG_THREE);
  const lstmCandidate = {
    preactivation: CANDIDATE_PREACTIVATION,
    value: Math.tanh(CANDIDATE_PREACTIVATION),
  };
  const retainedCell = forgetGate.value * previousCell;
  const lstmCandidateWrite = inputGate.value * lstmCandidate.value;
  const cellState = retainedCell + lstmCandidateWrite;
  const exposedCell = Math.tanh(cellState);

  return {
    input,
    previousHidden,
    previousCell,
    gru: {
      resetGate,
      updateGate,
      candidate: {
        inputProduct,
        resetState,
        recurrentProduct,
        bias: GRU_CANDIDATE_BIAS,
        preactivation: gruCandidatePreactivation,
        value: gruCandidateValue,
      },
      retainedState,
      candidateWrite: gruCandidateWrite,
      hiddenState: retainedState + gruCandidateWrite,
    },
    lstm: {
      forgetGate,
      inputGate,
      outputGate,
      candidate: lstmCandidate,
      retainedCell,
      candidateWrite: lstmCandidateWrite,
      cellState,
      exposedCell,
      hiddenState: outputGate.value * exposedCell,
    },
  };
}

export function traceGateCounterfactual(
  model: GatedModel,
  selectedGate: GatedCellGate,
  gateValue: number,
  trace: GatedRecurrentTrace = traceGatedRecurrent(),
): GateCounterfactual {
  if (!Number.isFinite(gateValue) || gateValue < 0 || gateValue > 1) {
    throw new Error("NN11 gate interventions must be between zero and one.");
  }

  if (model === "gru") {
    if (selectedGate !== "reset" && selectedGate !== "update") {
      throw new Error(`Gate ${selectedGate} does not belong to the GRU.`);
    }
    const reset = selectedGate === "reset" ? gateValue : trace.gru.resetGate.value;
    const update = selectedGate === "update" ? gateValue : trace.gru.updateGate.value;
    const candidate = Math.tanh(
      trace.gru.candidate.inputProduct
      + reset * trace.previousHidden
      + trace.gru.candidate.bias,
    );
    return {
      model,
      gate: selectedGate,
      gateValue,
      candidate,
      cellState: null,
      hiddenState: (1 - update) * trace.previousHidden + update * candidate,
    };
  }

  if (!["forget", "input", "output"].includes(selectedGate)) {
    throw new Error(`Gate ${selectedGate} does not belong to the LSTM.`);
  }
  const forget = selectedGate === "forget" ? gateValue : trace.lstm.forgetGate.value;
  const inputGate = selectedGate === "input" ? gateValue : trace.lstm.inputGate.value;
  const output = selectedGate === "output" ? gateValue : trace.lstm.outputGate.value;
  const cellState = forget * trace.previousCell + inputGate * trace.lstm.candidate.value;
  return {
    model,
    gate: selectedGate,
    gateValue,
    candidate: trace.lstm.candidate.value,
    cellState,
    hiddenState: output * Math.tanh(cellState),
  };
}
