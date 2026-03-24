/**
 * ==========================================================================
 * MemoryView — Hex Dump with PC and SP Highlighting
 * ==========================================================================
 *
 * Shows the ARM1's byte-addressable memory as a classic hex dump:
 * each row has 16 bytes displayed in hex, with an ASCII column on the right.
 *
 * PC and SP are highlighted so you can see where the program is executing
 * and where the stack is growing.
 */

import { useState } from "react";
import type { SimulatorState } from "../../simulator/types.js";

const BYTES_PER_ROW = 16;

/** Printable ASCII characters (0x20–0x7E); otherwise show '.'. */
function toAscii(byte: number): string {
  return byte >= 0x20 && byte <= 0x7e ? String.fromCharCode(byte) : ".";
}

interface MemoryRowProps {
  addr: number;
  bytes: number[];
  pc: number;
  sp: number;
  lastReads: Set<number>;
  lastWrites: Set<number>;
}

function MemoryRow({ addr, bytes, pc, sp, lastReads, lastWrites }: MemoryRowProps) {
  return (
    <div className="mem-row">
      <span className="mem-addr">
        {addr.toString(16).toUpperCase().padStart(8, "0")}
      </span>
      <div className="mem-hex-group">
        {bytes.map((byte, i) => {
          const byteAddr = addr + i;
          const isPc = (byteAddr & ~3) === (pc & ~3);   // word-aligned match
          const isSp = (byteAddr & ~3) === (sp & ~3);
          const isRead  = lastReads.has(byteAddr);
          const isWrite = lastWrites.has(byteAddr);
          return (
            <span
              key={i}
              className={[
                "mem-byte",
                isPc    ? "mem-pc"    : "",
                isSp    ? "mem-sp"    : "",
                isRead  ? "mem-read"  : "",
                isWrite ? "mem-write" : "",
                i === 7 ? "mem-gap" : "",
              ].filter(Boolean).join(" ")}
              title={`0x${byteAddr.toString(16).toUpperCase().padStart(8, "0")} = ${byte}`}
            >
              {byte.toString(16).toUpperCase().padStart(2, "0")}
            </span>
          );
        })}
      </div>
      <div className="mem-ascii">
        {bytes.map((byte, i) => {
          const byteAddr = addr + i;
          const isWrite = lastWrites.has(byteAddr);
          return (
            <span key={i} className={`mem-char ${isWrite ? "mem-write" : ""}`}>
              {toAscii(byte)}
            </span>
          );
        })}
      </div>
    </div>
  );
}

interface MemoryViewProps {
  state: SimulatorState;
  readMemory: (addr: number, count: number) => number[];
}

export function MemoryView({ state, readMemory }: MemoryViewProps) {
  const [viewAddr, setViewAddr] = useState(0);
  const [rowCount] = useState(32); // 32 rows × 16 bytes = 512 bytes visible

  const pc = state.pc;
  const sp = state.registers[13] ?? 0;

  // Collect addresses accessed by the last instruction.
  const lastTrace = state.traces.at(-1);
  const lastReads = new Set<number>();
  const lastWrites = new Set<number>();
  if (lastTrace) {
    for (const { address } of lastTrace.memoryReads) {
      for (let b = 0; b < 4; b++) lastReads.add(address + b);
    }
    for (const { address } of lastTrace.memoryWrites) {
      for (let b = 0; b < 4; b++) lastWrites.add(address + b);
    }
  }

  // Read memory for the visible window.
  const totalBytes = rowCount * BYTES_PER_ROW;
  const memBytes = readMemory(viewAddr, totalBytes);
  const rows = Array.from({ length: rowCount }, (_, i) => ({
    addr: viewAddr + i * BYTES_PER_ROW,
    bytes: memBytes.slice(i * BYTES_PER_ROW, (i + 1) * BYTES_PER_ROW),
  }));

  function jumpTo(addr: number) {
    // Align to row boundary.
    setViewAddr(Math.max(0, Math.floor(addr / BYTES_PER_ROW) * BYTES_PER_ROW));
  }

  return (
    <div className="memory-view">
      <header className="memory-header">
        <h2 className="panel-title">Memory</h2>
        <p className="panel-subtitle">
          4 KiB address space. Blue = PC word, orange = last read, red = last write.
        </p>
        <div className="memory-controls">
          <button className="mem-btn" onClick={() => jumpTo(0)}>
            Go to 0x0000
          </button>
          <button className="mem-btn" onClick={() => jumpTo(pc)}>
            Go to PC (0x{pc.toString(16).toUpperCase().padStart(4, "0")})
          </button>
          <button className="mem-btn" onClick={() => jumpTo(sp)}>
            Go to SP (0x{sp.toString(16).toUpperCase().padStart(4, "0")})
          </button>
          {lastTrace?.memoryReads[0] && (
            <button className="mem-btn mem-btn-read" onClick={() => jumpTo(lastTrace.memoryReads[0]!.address)}>
              Go to last read
            </button>
          )}
          {lastTrace?.memoryWrites[0] && (
            <button className="mem-btn mem-btn-write" onClick={() => jumpTo(lastTrace.memoryWrites[0]!.address)}>
              Go to last write
            </button>
          )}
        </div>
      </header>

      <div className="memory-dump" aria-label="Memory hex dump">
        <div className="mem-row mem-row-header">
          <span className="mem-addr">Address</span>
          <div className="mem-hex-group">
            {Array.from({ length: BYTES_PER_ROW }, (_, i) => (
              <span key={i} className={`mem-byte-header ${i === 7 ? "mem-gap" : ""}`}>
                +{i.toString(16).toUpperCase()}
              </span>
            ))}
          </div>
          <div className="mem-ascii">ASCII</div>
        </div>
        {rows.map(({ addr, bytes }) => (
          <MemoryRow
            key={addr}
            addr={addr}
            bytes={bytes}
            pc={pc}
            sp={sp}
            lastReads={lastReads}
            lastWrites={lastWrites}
          />
        ))}
      </div>

      <div className="memory-legend">
        <span className="legend-item mem-pc">PC word</span>
        <span className="legend-item mem-sp">SP word</span>
        <span className="legend-item mem-read">last read</span>
        <span className="legend-item mem-write">last write</span>
      </div>
    </div>
  );
}
