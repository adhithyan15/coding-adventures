export interface WasmState {
  readonly stack: readonly number[];
  readonly locals: readonly number[];
  readonly pc: number;
  readonly halted: boolean;
  readonly cycle: number;
}
