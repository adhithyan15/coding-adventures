/// <reference types="vite/client" />

// Vite's `?raw` import suffix returns the file contents as a string constant.
// This declaration tells TypeScript that any import ending in `?raw` resolves
// to a string, so modules like browser-transpiler.ts can import grammar files
// without type errors.
declare module "*.tokens?raw" {
  const content: string;
  export default content;
}

declare module "*.grammar?raw" {
  const content: string;
  export default content;
}
