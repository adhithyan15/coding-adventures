export interface TypeErrorDiagnostic {
  readonly message: string;
  readonly line: number;
  readonly column: number;
}

export interface TypeCheckResult<AST> {
  readonly typedAst: AST;
  readonly errors: readonly TypeErrorDiagnostic[];
  readonly ok: boolean;
}

export interface TypeChecker<ASTIn, ASTOut> {
  check(ast: ASTIn): TypeCheckResult<ASTOut>;
}

export type HookResult = unknown;
export type Hook<AST> = (node: AST, ...args: unknown[]) => HookResult;

const NOT_HANDLED = Symbol("type-checker:not-handled");

export abstract class GenericTypeChecker<AST> implements TypeChecker<AST, AST> {
  private readonly hooks = new Map<string, Hook<AST>[]>();
  private errors: TypeErrorDiagnostic[] = [];

  check(ast: AST): TypeCheckResult<AST> {
    this.errors = [];
    this.run(ast);
    return {
      typedAst: ast,
      errors: [...this.errors],
      ok: this.errors.length === 0,
    };
  }

  protected abstract run(ast: AST): void;

  protected abstract nodeKind(node: AST): string | null;

  protected locate(subject: unknown): [number, number] {
    void subject;
    return [1, 1];
  }

  protected registerHook(phase: string, kind: string, hook: Hook<AST>): void {
    const key = `${phase}:${this.normalizeKind(kind)}`;
    const existing = this.hooks.get(key) ?? [];
    existing.push(hook);
    this.hooks.set(key, existing);
  }

  protected dispatch(
    phase: string,
    node: AST,
    ...args: unknown[]
  ): HookResult {
    const normalizedKind = this.normalizeKind(this.nodeKind(node) ?? "");

    for (const key of [`${phase}:${normalizedKind}`, `${phase}:*`]) {
      for (const hook of this.hooks.get(key) ?? []) {
        const result = hook(node, ...args);
        if (result !== NOT_HANDLED) {
          return result;
        }
      }
    }

    return undefined;
  }

  protected notHandled(): symbol {
    return NOT_HANDLED;
  }

  protected error(message: string, subject: unknown): void {
    const [line, column] = this.locate(subject);
    this.errors.push({ message, line, column });
  }

  private normalizeKind(kind: string): string {
    let normalized = "";
    let lastWasUnderscore = false;

    for (const char of kind) {
      const isAlphaNumeric =
        (char >= "a" && char <= "z") ||
        (char >= "A" && char <= "Z") ||
        (char >= "0" && char <= "9");

      if (isAlphaNumeric) {
        normalized += char;
        lastWasUnderscore = false;
        continue;
      }

      if (!lastWasUnderscore) {
        normalized += "_";
        lastWasUnderscore = true;
      }
    }

    let start = 0;
    while (start < normalized.length && normalized[start] === "_") {
      start += 1;
    }

    let end = normalized.length;
    while (end > start && normalized[end - 1] === "_") {
      end -= 1;
    }

    return normalized.slice(start, end);
  }
}
