import { transpileLatticeInBrowser } from "@coding-adventures/lattice-transpiler/src/browser.js";
import latticeSource from "./app.lattice?raw";

const STYLE_ELEMENT_ID = "coding-adventures-lattice-styles";

export function installLatticeStyles(): void {
  if (document.getElementById(STYLE_ELEMENT_ID) !== null) {
    return;
  }

  try {
    const style = document.createElement("style");
    style.id = STYLE_ELEMENT_ID;
    style.textContent = transpileLatticeInBrowser(latticeSource);
    document.head.append(style);
  } catch (error) {
    console.error("Failed to install Lattice styles", error);
  }
}
