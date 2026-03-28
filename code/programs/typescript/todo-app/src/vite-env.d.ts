/// <reference types="vite/client" />

// Lattice stylesheet imports — the vite-plugin-lattice plugin transpiles
// .lattice files to CSS at build time. This declaration tells TypeScript
// that importing a .lattice file returns a string (the generated CSS).
declare module "*.lattice" {
  const css: string;
  export default css;
}
