import { describe, expect, it } from "vitest";
import {
  Circuit,
  SpiceError,
  SensResult,
  currentSource,
  resistor,
  sensDc,
  voltageSource,
} from "../src/index.js";

function expectClose(actual: number | undefined, expected: number): void {
  expect(actual).not.toBeUndefined();
  expect(actual!).toBeCloseTo(expected, 6);
}

function entry(
  result: SensResult,
  elementName: string,
  parameter: string,
) {
  const sensitivity = result.entry(elementName, parameter);
  expect(sensitivity).not.toBeUndefined();
  return sensitivity!;
}

describe("sensDc", () => {
  it("reports divider source and resistor sensitivities", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "vin", "0", 10.0));
    circuit.add(resistor("Rtop", "vin", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 1_000.0));

    const result = sensDc(circuit, "out");

    expect(result.outputNode).toBe("out");
    expectClose(result.nominalVoltage, 5.0);
    expect(result.entries).toHaveLength(3);
    expectClose(entry(result, "Vin", "voltage").sensitivity, 0.5);
    expectClose(entry(result, "Vin", "voltage").relativeSensitivity, 1.0);
    expectClose(
      entry(result, "Rtop", "resistanceOhms").sensitivity,
      -0.0025,
    );
    expectClose(
      entry(result, "Rtop", "resistanceOhms").relativeSensitivity,
      -0.5,
    );
    expectClose(
      entry(result, "Rbot", "resistanceOhms").sensitivity,
      0.0025,
    );
    expectClose(
      entry(result, "Rbot", "resistanceOhms").relativeSensitivity,
      0.5,
    );
  });

  it("reports current source and load resistance sensitivities", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("Iin", "0", "out", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = sensDc(circuit, "out");

    expectClose(result.nominalVoltage, 1.0);
    expectClose(entry(result, "Iin", "current").sensitivity, 1_000.0);
    expectClose(entry(result, "Iin", "current").relativeSensitivity, 1.0);
    expectClose(
      entry(result, "Rload", "resistanceOhms").sensitivity,
      1.0e-3,
    );
    expectClose(
      entry(result, "Rload", "resistanceOhms").relativeSensitivity,
      1.0,
    );
  });

  it("sorts entries by absolute relative sensitivity", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "vin", "0", 10.0));
    circuit.add(resistor("Rtop", "vin", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 1_000.0));

    const result = sensDc(circuit, "out");

    expect(result.entries[0].elementName).toBe("Vin");
    expect(Math.abs(result.entries[0].relativeSensitivity)).toBeGreaterThanOrEqual(
      Math.abs(result.entries[1].relativeSensitivity),
    );
  });

  it("rejects missing output nodes", () => {
    const circuit = new Circuit();

    expect(() => sensDc(circuit, "missing")).toThrowError(SpiceError);
    expect(() => sensDc(circuit, "missing")).toThrowError(
      "output node was not found in circuit",
    );
  });
});
