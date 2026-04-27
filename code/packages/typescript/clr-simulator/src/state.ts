export interface CLRState {
  readonly stack: readonly (number | null)[];
  readonly locals: readonly (number | null)[];
  readonly pc: number;
  readonly halted: boolean;
}
