# @coding-adventures/symbolic-vm

Pure TypeScript evaluator for `@coding-adventures/symbolic-ir` trees.

The VM is policy-free. A backend supplies name lookup, held heads, rewrite
rules, and per-head handlers.

```ts
import { ADD, app, int, sym } from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "@coding-adventures/symbolic-vm";

const vm = new VM(new SymbolicBackend());

vm.eval(app(ADD, [int(2), int(3)]));    // 5
vm.eval(app(ADD, [sym("x"), int(0)])); // x
```

The package has no host dependencies and is safe to use in browsers.
