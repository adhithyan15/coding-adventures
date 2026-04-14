export interface JVMState {
  readonly stack: readonly number[];
  readonly locals: readonly (number | null)[];
  readonly constants: readonly (number | string)[];
  readonly pc: number;
  readonly halted: boolean;
  readonly returnValue: number | null;
}
