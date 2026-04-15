export interface Intel4004State {
  readonly accumulator: number;
  readonly registers: readonly number[];
  readonly carry: boolean;
  readonly pc: number;
  readonly halted: boolean;
  readonly ram: readonly (readonly (readonly number[])[])[];
  readonly hwStack: readonly number[];
  readonly stackPointer: number;
}
