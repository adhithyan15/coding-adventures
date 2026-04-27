import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App.js";

describe("Nib web playground", () => {
  it("compiles a valid Nib example and shows HEX output", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Compile to Intel 4004" }));

    expect(screen.getByText("Compilation succeeded")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Intel HEX" })).toBeInTheDocument();
    expect(screen.getByLabelText("Intel HEX output")).toHaveTextContent(":");
  });

  it("shows type-checking errors for invalid programs", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Nib Source"), {
      target: {
        value: `fn main() {
    let value: u4 = true;
}`,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Compile to Intel 4004" }));

    expect(screen.getByText("Compilation failed")).toBeInTheDocument();
    expect(screen.getByLabelText("Diagnostics panel")).toHaveTextContent("type-check");
  });

  it("runs the compiled program in the simulator", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Compile to Intel 4004" }));
    fireEvent.click(screen.getByRole("button", { name: "Run to Halt" }));

    expect(screen.getByText("HALTED")).toBeInTheDocument();
    expect(screen.getByLabelText("Execution trace")).toHaveTextContent("BBL");
  });
});
