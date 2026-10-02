// @vitest-environment happy-dom

import { beforeEach, describe, expect, it } from "vitest";
// The shipped asset is intentionally plain browser JavaScript, not site build code.
// @ts-expect-error No TypeScript declaration is published for the browser asset.
import { enhancePipelineSteps } from "./data/assets/pipeline-steps.js";

function renderFixture(sibling = "ol"): void {
  document.body.innerHTML = `
    <div id="forme-pipeline-steps"><p>The complete pipeline.</p></div>
    <${sibling}>
      <li>Source</li>
      <li>Parse</li>
      <li>Emit</li>
    </${sibling}>
  `;
}

function steps(): HTMLLIElement[] {
  return [...document.querySelectorAll("li")];
}

describe("shipped pipeline-step enhancement", () => {
  beforeEach(() => renderFixture());

  it("preserves the complete fallback until a reader focuses a step", () => {
    expect(enhancePipelineSteps()).toBe(true);
    expect(steps().map(step => step.hidden)).toEqual([false, false, false]);

    const buttons = [...document.querySelectorAll("button")];
    expect(buttons.map(button => button.textContent)).toEqual([
      "Previous step",
      "Focus first step",
      "Show all steps",
    ]);
    buttons[1]?.click();
    expect(steps().map(step => step.hidden)).toEqual([false, true, true]);
    expect(document.querySelector('[aria-live="polite"]')?.textContent)
      .toBe("Step 1 of 3");

    buttons[2]?.click();
    expect(steps().map(step => step.hidden)).toEqual([false, false, false]);
  });

  it("is idempotent after activation", () => {
    expect(enhancePipelineSteps()).toBe(true);
    expect(enhancePipelineSteps()).toBe(false);
    expect(document.querySelectorAll("nav")).toHaveLength(1);
  });

  it("fails closed without mutating missing or malformed fallback structure", () => {
    renderFixture("ul");
    const malformed = document.body.innerHTML;
    expect(enhancePipelineSteps()).toBe(false);
    expect(document.body.innerHTML).toBe(malformed);

    document.body.innerHTML = "<ol><li>Unrelated</li></ol>";
    const missing = document.body.innerHTML;
    expect(enhancePipelineSteps()).toBe(false);
    expect(document.body.innerHTML).toBe(missing);
  });
});
