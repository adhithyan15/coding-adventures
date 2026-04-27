import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { App } from "../src/App.js";

beforeAll(() => {
  Object.defineProperty(window, "devicePixelRatio", {
    value: 1,
    configurable: true,
  });

  Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
    value: vi.fn(() => ({
      clearRect: vi.fn(),
      fillRect: vi.fn(),
      setTransform: vi.fn(),
    })),
    configurable: true,
  });
});

describe("App", () => {
  it("renders a barcode preview for valid default input", () => {
    const { container } = render(<App />);

    expect(screen.getByRole("heading", { name: "Code 39 Visualizer" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("CODE39-123")).toBeInTheDocument();
    expect(screen.getByText("The current input is valid Code 39 data.")).toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeNull();
    expect(screen.getByLabelText("encoded symbol alignment")).toBeInTheDocument();
    expect(screen.getAllByText("*").length).toBeGreaterThan(0);
  });

  it("normalizes lowercase input before rendering", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Value to encode"), {
      target: { value: "hello-39" },
    });

    expect(screen.getAllByText("HELLO-39").length).toBeGreaterThan(0);
    expect(screen.getByText("The current input is valid Code 39 data.")).toBeInTheDocument();
  });

  it("can switch the preview to the Canvas paint vm", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Canvas Paint VM" }));

    expect(screen.getByLabelText("barcode canvas preview")).toBeInTheDocument();
  });

  it("shows an error for unsupported input", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Value to encode"), {
      target: { value: "hello*" },
    });

    expect(screen.getByRole("alert")).toHaveTextContent('Input must not contain "*"');
    expect(screen.getByText("Fix the input to generate a barcode preview.")).toBeInTheDocument();
  });
});
