import { describe, it, expect, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import "@testing-library/jest-dom";
import { initI18n } from "@coding-adventures/ui-components";
import en from "../i18n/locales/en.json";
import {
  appState,
  createTemplate,
  createInstance,
} from "../state.js";
import { InstanceRunner } from "./InstanceRunner.js";
import type { TemplateItem } from "../types.js";

beforeAll(() => {
  initI18n({ en });
});

beforeEach(() => {
  appState.templates.length = 0;
  appState.instances.length = 0;
});

function makeRunner(items: TemplateItem[], navigate = vi.fn()) {
  const t = createTemplate(appState, "Test", "", items);
  const inst = createInstance(appState, t.id);
  const { rerender } = render(
    <InstanceRunner instanceId={inst.id} onNavigate={navigate} />,
  );
  return { inst, navigate, rerender };
}

describe("InstanceRunner", () => {
  it("renders template name as heading", () => {
    makeRunner([]);
    expect(screen.getByText("Test")).toBeInTheDocument();
  });

  it("shows empty message for template with no items", () => {
    makeRunner([]);
    expect(
      screen.getByText(/this checklist has no items/i),
    ).toBeInTheDocument();
  });

  it("renders check items", () => {
    makeRunner([{ id: "i1", type: "check", label: "Do the thing" }]);
    expect(screen.getByText("Do the thing")).toBeInTheDocument();
  });

  it("clicking a check item marks it checked", () => {
    makeRunner([{ id: "i1", type: "check", label: "Step 1" }]);
    const item = screen.getByRole("checkbox");
    expect(item).toHaveAttribute("aria-checked", "false");
    fireEvent.click(item);
    expect(item).toHaveAttribute("aria-checked", "true");
  });

  it("Complete button is disabled until all items are checked", () => {
    makeRunner([{ id: "i1", type: "check", label: "Step 1" }]);
    const completeBtn = screen.getByText(/complete checklist/i);
    expect(completeBtn).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox"));
    expect(completeBtn).not.toBeDisabled();
  });

  it("Complete button navigates to stats", () => {
    const navigate = vi.fn();
    const { inst } = makeRunner([], navigate);
    fireEvent.click(screen.getByText(/complete checklist/i));
    expect(navigate).toHaveBeenCalledWith(`/instance/${inst.id}/stats`);
  });

  it("renders decision item with Yes/No buttons", () => {
    makeRunner([
      {
        id: "d1",
        type: "decision",
        label: "Did it work?",
        yesBranch: [],
        noBranch: [],
      },
    ]);
    expect(screen.getByText("Did it work?")).toBeInTheDocument();
    expect(screen.getByText("Yes")).toBeInTheDocument();
    expect(screen.getByText("No")).toBeInTheDocument();
  });

  it("answering Yes reveals yes-branch items", () => {
    makeRunner([
      {
        id: "d1",
        type: "decision",
        label: "Did it work?",
        yesBranch: [{ id: "y1", type: "check", label: "Yes-branch step" }],
        noBranch: [{ id: "n1", type: "check", label: "No-branch step" }],
      },
    ]);
    fireEvent.click(screen.getByText("Yes"));
    expect(screen.getByText("Yes-branch step")).toBeInTheDocument();
    expect(screen.queryByText("No-branch step")).not.toBeInTheDocument();
  });

  it("Abandon navigates to stats as abandoned", () => {
    const navigate = vi.fn();
    const { inst } = makeRunner([], navigate);
    fireEvent.click(screen.getByText(/abandon/i));
    expect(navigate).toHaveBeenCalledWith(`/instance/${inst.id}/stats`);
    expect(inst.status).toBe("abandoned");
  });
});
