import type { IrProgram } from "@coding-adventures/compiler-ir";

export interface IrPass {
  readonly name: string;
  run(program: IrProgram): IrProgram;
}
