import { describe, it, expect } from "vitest";
import { Intel4004GateLevel } from "@coding-adventures/intel4004-gatelevel";
import { getBusicomROM } from "./busicom-rom.js";

describe("ROM debug", () => {
  it("should complete initialization and reach KEY_SCAN", () => {
    const cpu = new Intel4004GateLevel();
    cpu.loadProgram(getBusicomROM());

    // Run until we see RDR (0xEA) with acc=0 — meaning idle in scan loop
    let reachedScan = false;
    let stepCount = 0;
    for (let i = 0; i < 5000; i++) {
      const trace = cpu.step();
      stepCount++;
      if (trace.raw === 0xea) {
        console.log(`RDR at step ${stepCount}, acc=${cpu.accumulator}, PC=0x${cpu.pc.toString(16)}`);
        if (cpu.accumulator === 0) {
          reachedScan = true;
          break;
        }
      }
    }
    console.log(`Init took ${stepCount} steps, reached scan: ${reachedScan}`);
    expect(reachedScan).toBe(true);
  });

  it("should handle digit press 5", () => {
    const cpu = new Intel4004GateLevel();
    cpu.loadProgram(getBusicomROM());

    // Init
    for (let i = 0; i < 5000; i++) {
      const trace = cpu.step();
      if (trace.raw === 0xea && cpu.accumulator === 0) break;
    }
    console.log(`After init: PC=0x${cpu.pc.toString(16)}`);

    // Press 5
    cpu.romPort = 5;
    let keyRead = false;
    let stepCount = 0;
    for (let i = 0; i < 5000; i++) {
      const trace = cpu.step();
      stepCount++;

      if (trace.raw === 0xea) {
        if (!keyRead) {
          console.log(`First RDR at step ${stepCount}: acc=${cpu.accumulator}`);
          keyRead = true;
          cpu.romPort = 0; // Clear key after first read
        } else if (cpu.accumulator === 0) {
          console.log(`Idle RDR at step ${stepCount}`);
          break;
        }
      }

      // Log important instructions
      if (trace.address >= 0x040 && trace.address < 0x060) {
        console.log(`DIGIT_ENTRY: ${trace.address.toString(16)}: ${trace.mnemonic} acc=${cpu.accumulator}`);
      }
      if (trace.mnemonic.includes("WRM") && trace.address >= 0x040 && trace.address < 0x060) {
        console.log(`  >>> WRM at DIGIT_ENTRY`);
      }
    }

    console.log(`After key: RAM[1][0]=${cpu.ramData[0][1][0]}, RAM[0][0]=${cpu.ramData[0][0][0]}`);
    expect(cpu.ramData[0][1][0]).toBe(5);
  });
});
