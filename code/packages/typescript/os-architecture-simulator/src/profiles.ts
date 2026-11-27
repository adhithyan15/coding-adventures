import type { Effect, JsonValue, Profile, Stimulus, Transition } from "./core.ts";

function stringField(data: Stimulus["data"], key: string): string {
  const value = data[key];
  return typeof value === "string" ? value : "";
}

function positiveNumberField(data: Stimulus["data"], key: string): number {
  const value = data[key];
  if (typeof value === "number") return value;
  if (typeof value === "string" && value.trim().length > 0) return Number(value);
  return Number.NaN;
}

interface BatchJob { id: string; remaining: number }
interface BatchState { queued: BatchJob[]; active: BatchJob | null; completed: string[] }

/** Synthetic batch monitor: one service unit is consumed by each clock tick. */
export class BatchMonitorProfile implements Profile {
  readonly id = "synthetic-batch-monitor-v1";

  initialState(): JsonValue { return { queued: [], active: null, completed: [] }; }

  transition(value: JsonValue, tick: number, stimulus: Stimulus): Transition {
    const state = value as unknown as BatchState;
    const effects: Effect[] = [];
    if (stimulus.kind === "job.submit") {
      const id = stringField(stimulus.data, "id");
      const service = positiveNumberField(stimulus.data, "serviceTicks");
      if (!id || !Number.isSafeInteger(service) || service <= 0 || state.queued.some(job => job.id === id) || state.active?.id === id || state.completed.includes(id)) {
        return { state: state as unknown as JsonValue, effects: [{ kind: "job.rejected", data: { id, reason: "invalid-or-duplicate" } }] };
      }
      state.queued.push({ id, remaining: service });
      effects.push({ kind: "job.accepted", data: { id, queuePosition: String(state.queued.length) } });
    } else if (stimulus.kind === "clock.tick") {
      if (!state.active && state.queued.length > 0) {
        state.active = state.queued.shift()!;
        effects.push({ kind: "job.started", data: { id: state.active.id } });
      }
      if (state.active) {
        state.active.remaining--;
        effects.push({ kind: "job.serviced", data: { id: state.active.id, remaining: String(state.active.remaining) } });
        if (state.active.remaining === 0) {
          const id = state.active.id;
          state.completed.push(id);
          state.active = null;
          effects.push({ kind: "job.completed", data: { id } });
        }
      }
    } else {
      effects.push({ kind: "stimulus.ignored", data: { kind: stimulus.kind, reason: "unsupported-by-profile" } });
    }
    void tick;
    return { state: state as unknown as JsonValue, effects };
  }
}

interface Task { id: string; quantum: number; remaining: number; capability: string }
interface Message { to: string; body: JsonValue }
interface ProtectedSchedulerState { tasks: Task[]; cursor: number; messages: Message[] }

/** Synthetic modern policy: round-robin quanta and recipient capability checks. */
export class ProtectedMessageSchedulerProfile implements Profile {
  readonly id = "synthetic-protected-message-scheduler-v1";

  initialState(): JsonValue { return { tasks: [], cursor: 0, messages: [] }; }

  transition(value: JsonValue, tick: number, stimulus: Stimulus): Transition {
    const state = value as unknown as ProtectedSchedulerState;
    const effects: Effect[] = [];
    if (stimulus.kind === "task.create") {
      const id = stringField(stimulus.data, "id");
      const quantum = positiveNumberField(stimulus.data, "quantum");
      if (!id || !Number.isSafeInteger(quantum) || quantum <= 0 || state.tasks.some(task => task.id === id)) {
        return { state: state as unknown as JsonValue, effects: [{ kind: "task.rejected", data: { id, reason: "invalid-or-duplicate" } }] };
      }
      state.tasks.push({ id, quantum, remaining: quantum, capability: `cap:${id}` });
      effects.push({ kind: "task.created", data: { id, quantum: String(quantum) } });
    } else if (stimulus.kind === "message.send") {
      const to = stringField(stimulus.data, "to");
      const task = state.tasks.find(candidate => candidate.id === to);
      if (!task || stimulus.data.capability !== task.capability) {
        effects.push({ kind: "ipc.denied", data: { to, reason: "capability-check-failed" } });
      } else {
        const body = Object.hasOwn(stimulus.data, "body") ? stimulus.data.body : "";
        state.messages.push({ to, body });
        effects.push({ kind: "ipc.enqueued", data: { to, body } });
      }
    } else if (stimulus.kind === "clock.tick") {
      if (state.tasks.length > 0) {
        state.cursor %= state.tasks.length;
        const task = state.tasks[state.cursor]!;
        effects.push({ kind: "scheduler.dispatched", data: { id: task.id, quantumRemaining: String(task.remaining) } });
        const messageIndex = state.messages.findIndex(message => message.to === task.id);
        if (messageIndex >= 0) {
          const [message] = state.messages.splice(messageIndex, 1);
          effects.push({ kind: "ipc.delivered", data: { to: task.id, body: message!.body } });
        }
        task.remaining--;
        if (task.remaining === 0) {
          effects.push({ kind: "scheduler.preempted", data: { id: task.id } });
          task.remaining = task.quantum;
          state.cursor = (state.cursor + 1) % state.tasks.length;
        }
      }
    } else {
      effects.push({ kind: "stimulus.ignored", data: { kind: stimulus.kind, reason: "unsupported-by-profile" } });
    }
    void tick;
    return { state: state as unknown as JsonValue, effects };
  }
}
