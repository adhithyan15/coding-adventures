# Store

A Flux-like event-driven state store with middleware and a React hook. Zero dependencies.

## Architecture

```
dispatch(action) → middleware chain → reducer(state, action) → new state → listeners
```

- **State**: single immutable snapshot of the application
- **Actions**: plain objects with a `type` field describing what happened
- **Reducer**: pure function `(state, action) → newState`
- **Middleware**: intercepts dispatch for side effects (persistence, logging)
- **useStore hook**: React integration via useSyncExternalStore

## Usage

```typescript
import { Store, useStore } from "@coding-adventures/store";
import type { Action, Reducer } from "@coding-adventures/store";

interface AppState { count: number }

const reducer: Reducer<AppState> = (state, action) => {
  switch (action.type) {
    case "INCREMENT": return { count: state.count + 1 };
    default: return state;
  }
};

const store = new Store({ count: 0 }, reducer);

// In React:
function Counter() {
  const { count } = useStore(store);
  return (
    <button onClick={() => store.dispatch({ type: "INCREMENT" })}>
      Count: {count}
    </button>
  );
}
```

## Testing

```bash
npm install
npm run test
```

## Spec

See [`/code/specs/checklist-app.md`](/code/specs/checklist-app.md).
