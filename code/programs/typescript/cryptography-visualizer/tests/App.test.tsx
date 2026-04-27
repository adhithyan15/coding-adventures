import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../src/App.js";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("App", () => {
  it("renders with default Caesar cipher and HELLO WORLD", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Cryptography Visualizer" })).toBeInTheDocument();
    expect(screen.getByDisplayValue("HELLO WORLD")).toBeInTheDocument();

    // Default shift is 3: HELLO WORLD -> KHOOR ZRUOG
    const output = screen.getByTestId("ciphertext-output");
    expect(output).toHaveTextContent("KHOOR ZRUOG");
  });

  it("shows correct ciphertext for the default Caesar shift of 3", () => {
    render(<App />);

    const output = screen.getByTestId("ciphertext-output");
    expect(output).toHaveTextContent("KHOOR ZRUOG");
  });

  it("switching cipher to Atbash shows Atbash output", () => {
    render(<App />);

    // Switch to Atbash cipher
    fireEvent.change(screen.getByLabelText("Cipher"), {
      target: { value: "atbash" },
    });

    // Atbash of HELLO WORLD:
    // H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
    // W(22)->D(3), O(14)->L(11), R(17)->I(8), L(11)->O(14), D(3)->W(22)
    // HELLO WORLD -> SVOOL DLIOW
    const output = screen.getByTestId("ciphertext-output");
    expect(output).toHaveTextContent("SVOOL DLIOW");
  });

  it("changing shift updates ciphertext", () => {
    render(<App />);

    // Change shift to 1: HELLO WORLD -> IFMMP XPSME
    fireEvent.change(screen.getByRole("slider", { name: "shift amount" }), {
      target: { value: "1" },
    });

    const output = screen.getByTestId("ciphertext-output");
    expect(output).toHaveTextContent("IFMMP XPSME");
  });

  it("ROT13 button applies shift 13", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Apply ROT13" }));

    // ROT13 of HELLO WORLD -> URYYB JBEYQ
    const output = screen.getByTestId("ciphertext-output");
    expect(output).toHaveTextContent("URYYB JBEYQ");
  });

  it("handles empty input gracefully", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Plaintext"), {
      target: { value: "" },
    });

    // Empty input should produce empty output without errors
    const output = screen.getByTestId("ciphertext-output");
    // The output contains a non-breaking space when empty
    expect(output).toBeInTheDocument();
  });

  it("preserves non-alphabetic characters", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Plaintext"), {
      target: { value: "A1B2C3!" },
    });

    // With shift 3: A->D, B->E, C->F, digits and punctuation unchanged
    const output = screen.getByTestId("ciphertext-output");
    expect(output).toHaveTextContent("D1E2F3!");
  });

  it("shows substitution table with 26 cells", () => {
    render(<App />);

    const cells = screen.getAllByRole("cell");
    expect(cells.length).toBe(26);
  });

  it("shows frequency analysis panel for Caesar cipher", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "Frequency Analysis" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Brute Force Attack" })).toBeInTheDocument();
  });

  it("hides frequency analysis and brute force for Atbash", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("Cipher"), {
      target: { value: "atbash" },
    });

    expect(screen.queryByRole("heading", { name: "Frequency Analysis" })).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Brute Force Attack" })).not.toBeInTheDocument();
  });

  it("copies the current ciphertext to the clipboard", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Copy ciphertext" }));

    expect(writeText).toHaveBeenCalledWith("KHOOR ZRUOG");
    expect(await screen.findByText("Ciphertext copied to clipboard.")).toBeInTheDocument();
  });

  it("shows feedback when clipboard copy is unavailable", async () => {
    Object.defineProperty(window.navigator, "clipboard", {
      configurable: true,
      value: undefined,
    });

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Copy ciphertext" }));

    expect(await screen.findByText("Clipboard copy is unavailable in this browser.")).toBeInTheDocument();
  });
});
