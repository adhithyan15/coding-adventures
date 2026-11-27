import { Kernel, type SyscallEvent } from "./kernel.js";

type Pending =
  | { call: "open" | "list"; path: string }
  | { call: "read" | "close"; fd: number };

// Each step crosses the boundary once. "cat" keeps reading until it sees a
// zero-byte result, so the trace distinguishes a short read from EOF.
export class Session {
  private machine = new Kernel();
  private pending: Pending | undefined;
  private decoder = new TextDecoder();
  private trace: SyscallEvent[] = [];
  private consoleText = "";

  get kernel() { return this.machine; }
  get busy() { return this.pending !== undefined; }
  get events(): readonly SyscallEvent[] { return this.trace; }
  get output() { return this.consoleText; }

  submit(command: string): boolean {
    if (this.busy) return false;
    this.trace = [];
    this.consoleText = "";
    this.decoder = new TextDecoder();
    const [name, path, ...extra] = command.split(/[ \t\n\r\f\v]+/u).filter(Boolean);
    if (name === "cat" && path && extra.length === 0) {
      this.pending = { call: "open", path };
    } else if (name === "ls" && extra.length === 0) {
      this.pending = { call: "list", path: path ?? "/" };
    } else if (name) {
      this.consoleText = "usage: cat PATH or ls [PATH]\n";
    }
    return true;
  }

  step(): boolean {
    const pending = this.pending;
    if (!pending) return false;
    this.pending = undefined;

    switch (pending.call) {
      case "open": {
        const event = this.machine.open(pending.path);
        this.record(event);
        if ("fd" in event) this.pending = { call: "read", fd: event.fd };
        break;
      }
      case "read": {
        const { event, data } = this.machine.read(pending.fd, 8);
        this.record(event);
        if (data && data.length > 0) {
          this.consoleText += this.decoder.decode(data, { stream: true });
          this.pending = pending;
        } else {
          this.consoleText += this.decoder.decode();
          this.pending = { call: "close", fd: pending.fd };
        }
        break;
      }
      case "close":
        this.record(this.machine.close(pending.fd));
        break;
      case "list": {
        const event = this.machine.list(pending.path);
        this.record(event);
        if ("names" in event) this.consoleText = event.names.map(name => name + "\n").join("");
        break;
      }
    }
    return true;
  }

  run() {
    while (this.step()) {
      // The in-memory image is finite and each successful read consumes bytes.
    }
  }

  reset() {
    this.machine = new Kernel();
    this.pending = undefined;
    this.trace = [];
    this.consoleText = "";
  }

  private record(event: SyscallEvent) {
    this.trace.push(event);
    if ("error" in event && event.error) this.consoleText += `error: ${event.error}\n`;
  }
}
