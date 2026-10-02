/**
 * Progressive enhancement for the Hello, Forme pipeline walkthrough.
 *
 * The authored ordered list is the complete fallback. This module adds an
 * opt-in step explorer without fetching data, evaluating source text, or
 * replacing the list. Loading it with JavaScript disabled changes nothing.
 */

const TARGET_ID = "forme-pipeline-steps";

export function enhancePipelineSteps(root = document) {
  const fallback = root.getElementById(TARGET_ID);
  const list = fallback?.nextElementSibling;
  if (!(fallback instanceof HTMLElement) || !(list instanceof HTMLOListElement)) return false;
  const steps = [...list.children].filter(item => item instanceof HTMLLIElement);
  if (steps.length === 0 || fallback.dataset.enhanced === "true") return false;

  const controls = document.createElement("nav");
  controls.setAttribute("aria-label", "Forme pipeline step explorer");
  const status = document.createElement("span");
  status.setAttribute("aria-live", "polite");
  const previous = control("Previous step");
  const next = control("Next step");
  const showAll = control("Show all steps");
  controls.append(previous, next, showAll, status);
  fallback.append(controls);
  fallback.dataset.enhanced = "true";

  let selected = null;
  const render = () => {
    const showingAll = selected === null;
    steps.forEach((step, index) => { step.hidden = !showingAll && index !== selected; });
    previous.disabled = showingAll || selected === 0;
    next.disabled = !showingAll && selected === steps.length - 1;
    next.textContent = showingAll ? "Focus first step" : "Next step";
    showAll.disabled = showingAll;
    status.textContent = showingAll ? `Showing all ${steps.length} steps` : `Step ${selected + 1} of ${steps.length}`;
  };
  previous.addEventListener("click", () => { if (selected !== null && selected > 0) selected -= 1; render(); });
  next.addEventListener("click", () => { selected = selected === null ? 0 : Math.min(selected + 1, steps.length - 1); render(); });
  showAll.addEventListener("click", () => { selected = null; render(); });
  render();
  return true;
}

function control(label) {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  return button;
}

enhancePipelineSteps();
