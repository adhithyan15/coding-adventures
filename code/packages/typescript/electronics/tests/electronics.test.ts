import { describe, expect, test } from "vitest";
import { ConstantWaveform, SineWaveform } from "@coding-adventures/analog-waveform";
import {
  IdealResistor,
  analyzeDcResistor,
  analyzeVoltageDivider,
  sampleVoltageDrivenResistor,
} from "../src/index";

describe("IdealResistor", () => {
  test("obeys Ohm's law", () => {
    const resistor = new IdealResistor(50);
    expect(resistor.currentForVoltage(5)).toBeCloseTo(0.1, 10);
    expect(resistor.voltageForCurrent(0.1)).toBeCloseTo(5, 10);
  });

  test("computes power", () => {
    const resistor = new IdealResistor(100);
    expect(resistor.powerForVoltage(10)).toBeCloseTo(1, 10);
    expect(resistor.powerForCurrent(0.1)).toBeCloseTo(1, 10);
  });

  test("computes tolerance bounds", () => {
    const resistor = new IdealResistor(1000, 0.01);
    expect(resistor.minResistance()).toBeCloseTo(990, 10);
    expect(resistor.maxResistance()).toBeCloseTo(1010, 10);
  });
});

describe("analysis helpers", () => {
  test("analyzes a DC resistor", () => {
    const resistor = new IdealResistor(100);
    const result = analyzeDcResistor(5, resistor);
    expect(result.current).toBeCloseTo(0.05, 10);
    expect(result.power).toBeCloseTo(0.25, 10);
  });

  test("analyzes a voltage divider", () => {
    const top = new IdealResistor(1000);
    const bottom = new IdealResistor(1000);
    expect(analyzeVoltageDivider(5, top, bottom)).toBeCloseTo(2.5, 10);
  });

  test("samples waveform-driven resistor behavior", () => {
    const response = sampleVoltageDrivenResistor(
      new ConstantWaveform(5),
      new IdealResistor(50),
      1,
      5
    );

    expect(response.current[0]?.value).toBeCloseTo(0.1, 10);
    expect(response.instantaneousPower[0]?.value).toBeCloseTo(0.5, 10);
  });

  test("preserves phase for ideal resistor under sine excitation", () => {
    const response = sampleVoltageDrivenResistor(
      new SineWaveform(10, 1),
      new IdealResistor(5),
      0.25,
      2
    );

    expect(response.voltage[1]?.value).toBeCloseTo(10, 10);
    expect(response.current[1]?.value).toBeCloseTo(2, 10);
  });
});
