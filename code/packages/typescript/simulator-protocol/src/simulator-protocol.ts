export class StepTrace {
  readonly pcBefore: number;
  readonly pcAfter: number;
  readonly mnemonic: string;
  readonly description: string;

  constructor(
    pcBefore: number,
    pcAfter: number,
    mnemonic: string,
    description: string
  ) {
    this.pcBefore = pcBefore;
    this.pcAfter = pcAfter;
    this.mnemonic = mnemonic;
    this.description = description;
    Object.freeze(this);
  }
}

export interface ExecutionResultInit<StateT> {
  readonly halted: boolean;
  readonly steps: number;
  readonly finalState: StateT;
  readonly error: string | null;
  readonly traces?: readonly StepTrace[];
}

export class ExecutionResult<StateT> {
  readonly halted: boolean;
  readonly steps: number;
  readonly finalState: StateT;
  readonly error: string | null;
  readonly traces: readonly StepTrace[];

  constructor(init: ExecutionResultInit<StateT>) {
    this.halted = init.halted;
    this.steps = init.steps;
    this.finalState = init.finalState;
    this.error = init.error;
    this.traces = Object.freeze([...(init.traces ?? [])]);
    Object.freeze(this);
  }

  get ok(): boolean {
    return this.halted && this.error === null;
  }
}

export interface Simulator<StateT, TraceT = unknown> {
  load(program: Uint8Array): void;
  step(): TraceT;
  execute(program: Uint8Array, maxSteps?: number): ExecutionResult<StateT>;
  getState(): StateT;
  reset(): void;
}
