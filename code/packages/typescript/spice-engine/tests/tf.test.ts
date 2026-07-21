import { describe, expect, it } from "vitest";
import {
  bjt,
  Circuit,
  capacitor,
  cccs,
  ccvs,
  formatCornerTfTable,
  formatTfTable,
  currentSource,
  inductor,
  resistor,
  tf,
  tfCorners,
  vccs,
  voltageSource,
} from "../src/index.js";

function expectClose(actual: number, expected: number): void {
  expect(actual).toBeCloseTo(expected, 9);
}

describe("tf", () => {
  it("exposes a gain alias on transfer-function results", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = tf(circuit, "mid", "Vin");

    expectClose(result.gain(), 0.5);
  });

  it("reports gain and impedances for a voltage divider", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "vin", "0", 10.0));
    circuit.add(resistor("R1", "vin", "mid", 1_000.0));
    circuit.add(resistor("R2", "mid", "0", 1_000.0));

    const result = tf(circuit, "mid", "Vin");

    expectClose(result.transferRatio, 0.5);
    expectClose(result.inputImpedanceOhms, 2_000.0);
    expectClose(result.outputImpedanceOhms, 500.0);
  });

  it("uses BJT forward Early voltage to reduce output impedance", () => {
    function outputImpedance(forwardEarlyVoltage: number): number {
      const thermalVoltage = 0.02585;
      const circuit = new Circuit();
      circuit.add(voltageSource("Vcc", "vcc", "0", 5.0));
      circuit.add(voltageSource("Vin", "base", "0", thermalVoltage * Math.log(2.0)));
      circuit.add(resistor("Rload", "vcc", "out", 1_000.0));
      circuit.add(bjt("Q1", "out", "base", "0", "NPN", 25.85e-6, 100.0, thermalVoltage, 0.0, 0.0, 0.0, 0.0, 3.0, 1.11, forwardEarlyVoltage));
      return tf(circuit, "out", "Vin").outputImpedanceOhms;
    }

    expect(outputImpedance(10.0)).toBeLessThan(outputImpedance(0.0));
  });

  it("uses BJT forward emission coefficient to reduce gain and raise input impedance", () => {
    function transfer(forwardEmissionCoefficient: number) {
      const circuit = new Circuit();
      circuit.add(voltageSource("Vin", "base", "0", 0.65));
      circuit.add(resistor("Rload", "out", "0", 1_000.0));
      circuit.add(bjt("Q1", "out", "base", "0", "NPN", 1e-14, 100, 0.02585, 0, 0, 0, 0, 3, 1.11, 0, forwardEmissionCoefficient));
      return tf(circuit, "out", "Vin");
    }

    const ideal = transfer(1.0);
    const shaped = transfer(2.0);
    expect(Math.abs(shaped.gain())).toBeLessThan(Math.abs(ideal.gain()));
    expect(shaped.inputImpedanceOhms).toBeGreaterThan(ideal.inputImpedanceOhms);
  });

  it("formats stable text output tables for transfer-function results", () => {
    const result = {
      transferRatio: 0.5,
      inputImpedanceOhms: 2_000.0,
      outputImpedanceOhms: 500.0,
      gain: () => 0.5,
    };

    expect(formatTfTable(result)).toBe(
      "TransferRatio\tInputImpedance\tOutputImpedance\n" +
        "5.000000e-01\t2.000000e+03\t5.000000e+02\n",
    );
  });

  it("matches Thevenin values for an unequal divider", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 5.0));
    circuit.add(resistor("Rtop", "in", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 3_000.0));

    const result = tf(circuit, "out", "Vin");

    expectClose(result.gain(), 0.75);
    expectClose(result.inputImpedanceOhms, 4_000.0);
    expectClose(result.outputImpedanceOhms, 750.0);
  });

  it("runs transfer-function analysis at each named corner", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 10.0));
    circuit.add(resistor("Rtop", "in", "out", 1_000.0));
    circuit.add(resistor("Rbot", "out", "0", 1_000.0));

    const result = tfCorners(circuit, "out", "Vin", [
      { name: "nominal", overrides: [] },
      {
        name: "rbot-fast",
        overrides: [{ elementName: "Rbot", parameter: "resistance", value: 500.0 }],
      },
      {
        name: "rbot-slow",
        overrides: [{ elementName: "Rbot", parameter: "resistance", value: 2_000.0 }],
      },
    ]);

    expect(result.inputSource).toBe("Vin");
    expect(result.outputNode).toBe("out");
    expect(result.points.map((point) => point.cornerName)).toEqual([
      "nominal",
      "rbot-fast",
      "rbot-slow",
    ]);
    expectClose(result.points[0].result.gain(), 0.5);
    expectClose(result.points[1].result.gain(), 1.0 / 3.0);
    expectClose(result.points[2].result.gain(), 2.0 / 3.0);
    expectClose(result.points[0].result.inputImpedanceOhms, 2_000.0);
    expectClose(result.points[1].result.inputImpedanceOhms, 1_500.0);
    expectClose(result.points[2].result.inputImpedanceOhms, 3_000.0);
    expect(formatCornerTfTable(result)).toBe(
      "Corner\tTransferRatio\tInputImpedance\tOutputImpedance\n" +
      "nominal\t5.000000e-01\t2.000000e+03\t5.000000e+02\n" +
      "rbot-fast\t3.333333e-01\t1.500000e+03\t3.333333e+02\n" +
      "rbot-slow\t6.666667e-01\t3.000000e+03\t6.666667e+02\n",
    );
  });

  it("reports current-source transimpedance", () => {
    const circuit = new Circuit();
    circuit.add(currentSource("Iin", "0", "out", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 2_000.0));

    const result = tf(circuit, "out", "Iin");

    expectClose(result.transferRatio, 2_000.0);
    expectClose(result.inputImpedanceOhms, 2_000.0);
    expectClose(result.outputImpedanceOhms, 2_000.0);
  });

  it("treats capacitors as open and inductors as shorts at DC small signal", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(capacitor("Cblock", "in", "blocked", 1.0e-6));
    circuit.add(resistor("Rblocked", "blocked", "0", 1_000.0));
    circuit.add(inductor("Lshort", "in", "out", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = tf(circuit, "out", "Vin");

    expect(result.gain()).toBeCloseTo(1.0, 9);
    expect(result.outputImpedanceOhms).toBeLessThan(1.0e-6);
  });

  it("reports VCCS transconductance gain", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(vccs("Gm", "0", "out", "in", "0", 2.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = tf(circuit, "out", "Vin");

    expectClose(result.gain(), 2.0);
    expectClose(result.outputImpedanceOhms, 1_000.0);
  });

  it("reports CCCS current gain through a sensing voltage source", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(cccs("F1", "0", "out", "Vsense", 2.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = tf(circuit, "out", "Vin");

    expectClose(result.gain(), 2.0);
    expectClose(result.outputImpedanceOhms, 1_000.0);
  });

  it("reports CCVS transresistance gain through a sensing voltage source", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vin", "in", "0", 1.0));
    circuit.add(resistor("Rsense", "in", "sense", 1_000.0));
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(ccvs("H1", "out", "0", "Vsense", 2_000.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    const result = tf(circuit, "out", "Vin");

    expectClose(result.gain(), 2.0);
    expectClose(result.outputImpedanceOhms, 0.0);
  });

  it("rejects missing output nodes", () => {
    const circuit = new Circuit();

    expect(() => tf(circuit, "missing", "Vin")).toThrowError(
      "output node was not found",
    );
  });

  it("rejects non-source input elements", () => {
    const circuit = new Circuit();
    circuit.add(resistor("Rin", "in", "0", 1_000.0));

    expect(() => tf(circuit, "in", "Rin")).toThrowError(
      "input element must be an independent voltage or current source",
    );
  });

  it("rejects VCCS elements as input sources", () => {
    const circuit = new Circuit();
    circuit.add(vccs("Gm", "0", "out", "in", "0", 1.0e-3));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => tf(circuit, "out", "Gm")).toThrowError(
      "input element must be an independent voltage or current source",
    );
  });

  it("rejects CCCS elements as input sources", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(cccs("F1", "0", "out", "Vsense", 1.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => tf(circuit, "out", "F1")).toThrowError(
      "input element must be an independent voltage or current source",
    );
  });

  it("rejects CCVS elements as input sources", () => {
    const circuit = new Circuit();
    circuit.add(voltageSource("Vsense", "sense", "0", 0.0));
    circuit.add(ccvs("H1", "out", "0", "Vsense", 1_000.0));
    circuit.add(resistor("Rload", "out", "0", 1_000.0));

    expect(() => tf(circuit, "out", "H1")).toThrowError(
      "input element must be an independent voltage or current source",
    );
  });
});
