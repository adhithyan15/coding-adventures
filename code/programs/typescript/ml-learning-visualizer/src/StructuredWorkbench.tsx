import { HopfieldWorkbench } from "./HopfieldWorkbench.js";

export function StructuredWorkbench() {
  return (
    <div className="structured-workbench">
      <nav className="structured-lab-switch" aria-label="Structured and memory learning lab">
        <button aria-pressed="true" type="button">Hopfield memory</button>
      </nav>
      <HopfieldWorkbench />
    </div>
  );
}
