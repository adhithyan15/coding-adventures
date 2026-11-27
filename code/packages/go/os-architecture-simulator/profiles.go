package ossimulator

import (
	"fmt"
	"math"
	"strconv"
)

const (
	BatchMonitorID       = "synthetic-batch-monitor-v1"
	ProtectedSchedulerID = "synthetic-protected-message-scheduler-v1"
)

// BatchMonitorProfile is the synthetic queue-order policy from HOS03.
type BatchMonitorProfile struct{}

func (BatchMonitorProfile) ID() string { return BatchMonitorID }
func (BatchMonitorProfile) InitialState() any {
	return map[string]any{"queued": []any{}, "active": nil, "completed": []any{}}
}

func (BatchMonitorProfile) Transition(value any, _ uint64, stimulus Stimulus) (any, []Effect, error) {
	// Work on the isolated state copy supplied by Machine so validation failure
	// cannot mutate the previously committed batch queue.
	state, ok := value.(map[string]any)
	if !ok {
		return nil, nil, fmt.Errorf("batch state must be an object")
	}
	queued, _ := state["queued"].([]any)
	completed, _ := state["completed"].([]any)
	active, _ := state["active"].(map[string]any)
	effects := []Effect{}
	switch stimulus.Kind {
	case "job.submit":
		id := stringField(stimulus.Data, "id")
		service := positiveNumber(stimulus.Data["serviceTicks"])
		duplicate := false
		for _, raw := range queued {
			job, _ := raw.(map[string]any)
			if job["id"] == id {
				duplicate = true
			}
		}
		if active != nil && active["id"] == id {
			duplicate = true
		}
		for _, done := range completed {
			if done == id {
				duplicate = true
			}
		}
		if id == "" || !positiveInteger(service) || duplicate {
			effects = append(effects, effect("job.rejected", map[string]any{"id": id, "reason": "invalid-or-duplicate"}))
			break
		}
		queued = append(queued, map[string]any{"id": id, "remaining": service})
		effects = append(effects, effect("job.accepted", map[string]any{"id": id, "queuePosition": strconv.Itoa(len(queued))}))
	case "clock.tick":
		if active == nil && len(queued) > 0 {
			active, _ = queued[0].(map[string]any)
			queued = queued[1:]
			effects = append(effects, effect("job.started", map[string]any{"id": active["id"]}))
		}
		if active != nil {
			remaining, _ := number(active["remaining"])
			remaining--
			active["remaining"] = remaining
			effects = append(effects, effect("job.serviced", map[string]any{"id": active["id"], "remaining": wholeNumberString(remaining)}))
			if remaining == 0 {
				id := active["id"]
				completed = append(completed, id)
				active = nil
				effects = append(effects, effect("job.completed", map[string]any{"id": id}))
			}
		}
	default:
		effects = append(effects, effect("stimulus.ignored", map[string]any{"kind": stimulus.Kind, "reason": "unsupported-by-profile"}))
	}
	state["queued"], state["completed"] = queued, completed
	if active == nil {
		state["active"] = nil
	} else {
		state["active"] = active
	}
	return state, effects, nil
}

// ProtectedMessageSchedulerProfile is the synthetic round-robin/IPC policy.
type ProtectedMessageSchedulerProfile struct{}

func (ProtectedMessageSchedulerProfile) ID() string { return ProtectedSchedulerID }
func (ProtectedMessageSchedulerProfile) InitialState() any {
	return map[string]any{"tasks": []any{}, "cursor": 0, "messages": []any{}}
}

func (ProtectedMessageSchedulerProfile) Transition(value any, _ uint64, stimulus Stimulus) (any, []Effect, error) {
	state, ok := value.(map[string]any)
	if !ok {
		return nil, nil, fmt.Errorf("scheduler state must be an object")
	}
	tasks, _ := state["tasks"].([]any)
	messages, _ := state["messages"].([]any)
	cursorNumber, _ := number(state["cursor"])
	cursor := int(cursorNumber)
	effects := []Effect{}
	switch stimulus.Kind {
	case "task.create":
		id := stringField(stimulus.Data, "id")
		quantum := positiveNumber(stimulus.Data["quantum"])
		duplicate := false
		for _, raw := range tasks {
			task, _ := raw.(map[string]any)
			if task["id"] == id {
				duplicate = true
			}
		}
		if id == "" || !positiveInteger(quantum) || duplicate {
			effects = append(effects, effect("task.rejected", map[string]any{"id": id, "reason": "invalid-or-duplicate"}))
			break
		}
		tasks = append(tasks, map[string]any{"id": id, "quantum": quantum, "remaining": quantum, "capability": "cap:" + id})
		effects = append(effects, effect("task.created", map[string]any{"id": id, "quantum": wholeNumberString(quantum)}))
	case "message.send":
		to := stringField(stimulus.Data, "to")
		var receiver map[string]any
		for _, raw := range tasks {
			task, _ := raw.(map[string]any)
			if task["id"] == to {
				receiver = task
				break
			}
		}
		if receiver == nil || stimulus.Data["capability"] != receiver["capability"] {
			effects = append(effects, effect("ipc.denied", map[string]any{"to": to, "reason": "capability-check-failed"}))
		} else {
			body, exists := stimulus.Data["body"]
			if !exists {
				body = ""
			}
			messages = append(messages, map[string]any{"to": to, "body": body})
			effects = append(effects, effect("ipc.enqueued", map[string]any{"to": to, "body": body}))
		}
	case "clock.tick":
		if len(tasks) > 0 {
			cursor %= len(tasks)
			task, _ := tasks[cursor].(map[string]any)
			id := task["id"]
			remaining, _ := number(task["remaining"])
			effects = append(effects, effect("scheduler.dispatched", map[string]any{"id": id, "quantumRemaining": wholeNumberString(remaining)}))
			for index, raw := range messages {
				message, _ := raw.(map[string]any)
				if message["to"] == id {
					messages = append(messages[:index], messages[index+1:]...)
					effects = append(effects, effect("ipc.delivered", map[string]any{"to": id, "body": message["body"]}))
					break
				}
			}
			remaining--
			task["remaining"] = remaining
			if remaining == 0 {
				effects = append(effects, effect("scheduler.preempted", map[string]any{"id": id}))
				task["remaining"] = task["quantum"]
				cursor = (cursor + 1) % len(tasks)
			}
		}
	default:
		effects = append(effects, effect("stimulus.ignored", map[string]any{"kind": stimulus.Kind, "reason": "unsupported-by-profile"}))
	}
	state["tasks"], state["cursor"], state["messages"] = tasks, cursor, messages
	return state, effects, nil
}

func effect(kind string, data map[string]any) Effect { return Effect{Kind: kind, Data: data} }
func stringField(data map[string]any, key string) string {
	value, _ := data[key].(string)
	return value
}
func number(value any) (float64, bool) {
	switch item := value.(type) {
	case float64:
		return item, true
	case int:
		return float64(item), true
	case string:
		parsed, err := strconv.ParseFloat(item, 64)
		return parsed, err == nil
	default:
		return 0, false
	}
}
func positiveNumber(value any) float64 { result, _ := number(value); return result }
func positiveInteger(value float64) bool {
	return !math.IsNaN(value) && !math.IsInf(value, 0) && value > 0 && value <= 9_007_199_254_740_991 && math.Trunc(value) == value
}

func wholeNumberString(value float64) string { return strconv.FormatFloat(value, 'f', 0, 64) }
