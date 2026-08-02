import { useState } from "react";
import { GradientFlowWorkbench } from "./GradientFlowWorkbench.js";
import { InitializationWorkbench } from "./InitializationWorkbench.js";

export function DeepTrainingWorkbench() {
  const [lab, setLab] = useState<"initialization" | "gradient-flow">("initialization");
  return (
    <div className="deep-training-workbench">
      <nav className="deep-training-switch" aria-label="Deep training learning lab">
        <button aria-pressed={lab === "initialization"} type="button" onClick={() => setLab("initialization")}>Initialization</button>
        <button aria-pressed={lab === "gradient-flow"} type="button" onClick={() => setLab("gradient-flow")}>Gradient flow</button>
      </nav>
      {lab === "initialization" ? <InitializationWorkbench /> : <GradientFlowWorkbench />}
    </div>
  );
}
