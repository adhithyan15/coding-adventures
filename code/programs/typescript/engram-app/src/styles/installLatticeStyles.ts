import { transpileLattice } from "@coding-adventures/lattice-transpiler";
import latticeSource from "./app.lattice?raw";

const STYLE_ELEMENT_ID = "engram-lattice-styles";

export function installLatticeStyles(): void {
  if (document.getElementById(STYLE_ELEMENT_ID)) {
    return;
  }

  try {
    const style = document.createElement("style");
    style.id = STYLE_ELEMENT_ID;
    style.textContent = transpileLattice(latticeSource);
    document.head.append(style);
  } catch (error) {
    console.error("Failed to install Engram Lattice styles", error);
  }
}
