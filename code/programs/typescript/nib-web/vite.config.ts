import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { latticePlugin } from "../../../packages/typescript/vite-plugin-lattice/src/plugin.ts";

const alias = {
  "@coding-adventures/compiler-ir": fileURLToPath(new URL("../../../packages/typescript/compiler-ir/src/index.ts", import.meta.url)),
  "@coding-adventures/compiler-source-map": fileURLToPath(new URL("../../../packages/typescript/compiler-source-map/src/index.ts", import.meta.url)),
  "@coding-adventures/directed-graph": fileURLToPath(new URL("../../../packages/typescript/directed-graph/src/index.ts", import.meta.url)),
  "@coding-adventures/grammar-tools": fileURLToPath(new URL("../../../packages/typescript/grammar-tools/src/index.ts", import.meta.url)),
  "@coding-adventures/intel-4004-assembler": fileURLToPath(new URL("../../../packages/typescript/intel-4004-assembler/src/index.ts", import.meta.url)),
  "@coding-adventures/intel-4004-ir-validator": fileURLToPath(new URL("../../../packages/typescript/intel-4004-ir-validator/src/index.ts", import.meta.url)),
  "@coding-adventures/intel-4004-packager": fileURLToPath(new URL("../../../packages/typescript/intel-4004-packager/src/index.ts", import.meta.url)),
  "@coding-adventures/intel4004-simulator": fileURLToPath(new URL("../../../packages/typescript/intel4004-simulator/src/index.ts", import.meta.url)),
  "@coding-adventures/ir-optimizer": fileURLToPath(new URL("../../../packages/typescript/ir-optimizer/src/index.ts", import.meta.url)),
  "@coding-adventures/ir-to-intel-4004-compiler": fileURLToPath(new URL("../../../packages/typescript/ir-to-intel-4004-compiler/src/index.ts", import.meta.url)),
  "@coding-adventures/lattice-ast-to-css": fileURLToPath(new URL("../../../packages/typescript/lattice-ast-to-css/src/index.ts", import.meta.url)),
  "@coding-adventures/lattice-lexer": fileURLToPath(new URL("../../../packages/typescript/lattice-lexer/src/index.ts", import.meta.url)),
  "@coding-adventures/lattice-parser": fileURLToPath(new URL("../../../packages/typescript/lattice-parser/src/index.ts", import.meta.url)),
  "@coding-adventures/lattice-transpiler": fileURLToPath(new URL("../../../packages/typescript/lattice-transpiler/src/index.ts", import.meta.url)),
  "@coding-adventures/lexer": fileURLToPath(new URL("../../../packages/typescript/lexer/src/index.ts", import.meta.url)),
  "@coding-adventures/nib-ir-compiler": fileURLToPath(new URL("../../../packages/typescript/nib-ir-compiler/src/index.ts", import.meta.url)),
  "@coding-adventures/nib-type-checker": fileURLToPath(new URL("../../../packages/typescript/nib-type-checker/src/index.ts", import.meta.url)),
  "@coding-adventures/parser": fileURLToPath(new URL("../../../packages/typescript/parser/src/index.ts", import.meta.url)),
  "@coding-adventures/simulator-protocol": fileURLToPath(new URL("../../../packages/typescript/simulator-protocol/src/index.ts", import.meta.url)),
  "@coding-adventures/state-machine": fileURLToPath(new URL("../../../packages/typescript/state-machine/src/index.ts", import.meta.url)),
  "@coding-adventures/type-checker-protocol": fileURLToPath(new URL("../../../packages/typescript/type-checker-protocol/src/index.ts", import.meta.url)),
};

export default defineConfig({
  plugins: [latticePlugin(), react()],
  base: "/coding-adventures/nib-web/",
  resolve: {
    alias,
  },
});
