import { FunctionTypeStatus, IirFunction, IirModule, Types } from "@coding-adventures/interpreter-ir";
import { BackendRegistry, type Artifact } from "@coding-adventures/codegen-core";
import { BuiltinRegistry, VMCore, type JitHandler, type VMValue } from "@coding-adventures/vm-core";

export interface JitBackend {
  compileCallable(fn: IirFunction, mod: IirModule, sourceVm: VMCore): JitHandler;
}

export class PureVmBackend implements JitBackend {
  compileCallable(fn: IirFunction, mod: IirModule, sourceVm: VMCore): JitHandler {
    const builtins = new BuiltinRegistry(false);
    for (const [name, handler] of sourceVm.builtins.entries()) builtins.register(name, handler);
    return (args: VMValue[]) => new VMCore({ builtins, profilerEnabled: false, u8Wrap: fn.returnType === Types.U8 }).execute(mod, fn.name, args);
  }
}

export class JITCore {
  constructor(readonly vm: VMCore, readonly backend: JitBackend = new PureVmBackend(), readonly registry = BackendRegistry.default()) {}
  executeWithJit(mod: IirModule, fn = mod.entryPoint, args: VMValue[] = []): VMValue {
    this.compileReadyFunctions(mod);
    return this.vm.execute(mod, fn, args);
  }
  compileReadyFunctions(mod: IirModule): string[] {
    const compiled: string[] = [];
    for (const fn of mod.functions) {
      if (this.shouldCompile(fn)) {
        this.vm.registerJitHandler(fn.name, this.backend.compileCallable(fn, mod, this.vm));
        compiled.push(fn.name);
      }
    }
    return compiled;
  }
  emit(mod: IirModule, target: string): Artifact { return this.registry.compile(mod, target); }
  shouldCompile(fn: IirFunction): boolean {
    if (fn.typeStatus === FunctionTypeStatus.FullyTyped) return true;
    if (fn.typeStatus === FunctionTypeStatus.PartiallyTyped) return fn.callCount >= 10;
    return fn.callCount >= 100;
  }
}
