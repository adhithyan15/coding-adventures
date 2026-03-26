import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "../src/App.js";

describe("App", () => {
  it("renders a barcode preview for valid default input", () => {
    const { container } = render(<App />);

    expect(screen.getByRole("heading", { name: "Code 39 Visualizer" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("CODE39-123")).toBeInTheDocument();
    expect(screen.getByText("The current input is valid Code 39 data.")).toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("normalizes lowercase input before rendering", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Value to encode"), {
      target: { value: "hello-39" },
    });

    expect(screen.getAllByText("HELLO-39").length).toBeGreaterThan(0);
    expect(screen.getByText("The current input is valid Code 39 data.")).toBeInTheDocument();
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
