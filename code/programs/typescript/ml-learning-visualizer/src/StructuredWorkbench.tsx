import { useState } from "react";
import { HopfieldWorkbench } from "./HopfieldWorkbench.js";
import { MessagePassingWorkbench } from "./MessagePassingWorkbench.js";

export function StructuredWorkbench() {
  const [lab, setLab] = useState<"hopfield" | "message">("hopfield");
  return (
    <div className="structured-workbench">
      <nav className="structured-lab-switch" aria-label="Structured and memory learning lab">
        <button aria-pressed={lab === "hopfield"} type="button" onClick={() => setLab("hopfield")}>Hopfield memory</button>
        <button aria-pressed={lab === "message"} type="button" onClick={() => setLab("message")}>Message passing</button>
      </nav>
      {lab === "hopfield" ? <HopfieldWorkbench /> : <MessagePassingWorkbench />}
    </div>
  );
}
