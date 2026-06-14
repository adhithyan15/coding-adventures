import type { IirInstr, IirModule } from "@coding-adventures/interpreter-ir";

export interface Artifact {
  target: string;
  format: string;
  body: string;
  metadata: Record<string, unknown>;
}

export interface Backend {
  readonly target: string;
  compile(mod: IirModule): Artifact;
}

export class TextBackend implements Backend {
  readonly target: string;
  constructor(target: string) {
    this.target = target;
  }
  compile(mod: IirModule): Artifact {
    const lines = [`; LANG target=${this.target} module=${mod.name} language=${mod.language}`, `.entry ${mod.entryPoint}`];
    for (const fn of mod.functions) {
      const params = fn.params.map((param) => `${param.name}:${param.type}`).join(" ");
      lines.push("", `.function ${fn.name}${params.length > 0 ? ` ${params}` : ""} -> ${fn.returnType}`);
      fn.instructions.forEach((instr, index) => lines.push(`  ${index.toString().padStart(4, "0")}  ${formatInstr(instr)}`));
      lines.push(".end");
    }
    return {
      target: this.target,
      format: `${this.target}-lang-ir-text`,
      body: `${lines.join("\n")}\n`,
      metadata: { functions: mod.functionNames(), entry_point: mod.entryPoint },
    };
  }
}

export class BackendRegistry {
  static readonly defaultTargets = ["pure_vm", "jvm", "clr", "wasm"] as const;
  private readonly backends = new Map<string, Backend>();
  static default(): BackendRegistry {
    const registry = new BackendRegistry();
    for (const target of BackendRegistry.defaultTargets) registry.register(new TextBackend(target));
    return registry;
  }
  register(backend: Backend): void { this.backends.set(backend.target, backend); }
  fetch(target: string): Backend {
    const backend = this.backends.get(target);
    if (backend === undefined) throw new Error(`unknown backend target: ${target}`);
    return backend;
  }
  compile(mod: IirModule, target: string): Artifact { return this.fetch(target).compile(mod); }
  targets(): string[] { return [...this.backends.keys()]; }
}

function formatInstr(instr: IirInstr): string {
  const dest = instr.dest === null ? "" : `${instr.dest} = `;
  const args = instr.srcs.map((src) => (typeof src === "string" ? JSON.stringify(src) : String(src))).join(", ");
  const type = instr.typeHint === null ? "" : ` : ${instr.typeHint}`;
  return `${dest}${instr.op}(${args})${type}`;
}
