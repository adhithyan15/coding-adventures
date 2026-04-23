import { NibType } from "./types.js";

export interface SymbolRecord {
  readonly name: string;
  readonly nibType: NibType | null;
  readonly isConst?: boolean;
  readonly isStatic?: boolean;
  readonly isFn?: boolean;
  readonly fnParams?: ReadonlyArray<[string, NibType]>;
  readonly fnReturnType?: NibType | null;
}

export class ScopeChain {
  private readonly scopes: Array<Map<string, SymbolRecord>> = [new Map()];

  push(): void {
    this.scopes.push(new Map());
  }

  pop(): void {
    if (this.scopes.length <= 1) {
      throw new Error("Cannot pop the global scope.");
    }
    this.scopes.pop();
  }

  define(name: string, symbol: SymbolRecord): void {
    this.scopes[this.scopes.length - 1].set(name, symbol);
  }

  defineGlobal(name: string, symbol: SymbolRecord): void {
    this.scopes[0].set(name, symbol);
  }

  lookup(name: string): SymbolRecord | null {
    for (let i = this.scopes.length - 1; i >= 0; i -= 1) {
      const symbol = this.scopes[i].get(name);
      if (symbol) {
        return symbol;
      }
    }
    return null;
  }
}
