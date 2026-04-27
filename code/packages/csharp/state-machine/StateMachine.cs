namespace CodingAdventures.StateMachine;

public delegate void TransitionAction(string source, string @event, string target);

public sealed record TransitionRecord(string Source, string Event, string Target, string ActionName);
public sealed record ModeTransitionRecord(string FromMode, string Trigger, string ToMode);
public sealed record PDATransition(string Source, string? Event, string StackRead, string Target, IReadOnlyList<string> StackPush);
public sealed record PDATraceEntry(string Source, string? Event, string StackRead, string Target, IReadOnlyList<string> StackPush, IReadOnlyList<string> StackAfter);

public sealed class DFA
{
    private readonly Dictionary<(string State, string Event), string> _transitions;
    private readonly Dictionary<(string State, string Event), TransitionAction> _actions;
    private readonly List<TransitionRecord> _trace = [];

    public DFA(
        IEnumerable<string> states,
        IEnumerable<string> alphabet,
        IDictionary<(string State, string Event), string> transitions,
        string initial,
        IEnumerable<string> accepting,
        IDictionary<(string State, string Event), TransitionAction>? actions = null)
    {
        States = states.ToHashSet(StringComparer.Ordinal);
        Alphabet = alphabet.ToHashSet(StringComparer.Ordinal);
        Accepting = accepting.ToHashSet(StringComparer.Ordinal);
        if (States.Count == 0)
        {
            throw new ArgumentException("states set must be non-empty");
        }

        if (!States.Contains(initial))
        {
            throw new ArgumentException($"Initial state '{initial}' is not in the states set");
        }

        if (!Accepting.IsSubsetOf(States))
        {
            throw new ArgumentException("Accepting states must be a subset of states");
        }

        _transitions = new Dictionary<(string State, string Event), string>(transitions);
        foreach (var (key, target) in _transitions)
        {
            if (!States.Contains(key.State) || !States.Contains(target))
            {
                throw new ArgumentException("Transition source and target must be in states");
            }

            if (!Alphabet.Contains(key.Event))
            {
                throw new ArgumentException("Transition events must be in alphabet");
            }
        }

        _actions = actions is null ? [] : new Dictionary<(string State, string Event), TransitionAction>(actions);
        Initial = initial;
        CurrentState = initial;
    }

    public IReadOnlySet<string> States { get; }
    public IReadOnlySet<string> Alphabet { get; }
    public IReadOnlySet<string> Accepting { get; }
    public string Initial { get; }
    public string CurrentState { get; private set; }
    public IReadOnlyDictionary<(string State, string Event), string> Transitions => new Dictionary<(string State, string Event), string>(_transitions);
    public IReadOnlyList<TransitionRecord> Trace => _trace.ToList();

    public string Process(string @event)
    {
        if (!_transitions.TryGetValue((CurrentState, @event), out var target))
        {
            throw new InvalidOperationException($"No transition defined for ({CurrentState}, {@event})");
        }

        _actions.TryGetValue((CurrentState, @event), out var action);
        action?.Invoke(CurrentState, @event, target);
        _trace.Add(new TransitionRecord(CurrentState, @event, target, action?.Method.Name ?? string.Empty));
        CurrentState = target;
        return target;
    }

    public string Process(IEnumerable<string> events)
    {
        var state = CurrentState;
        foreach (var @event in events)
        {
            state = Process(@event);
        }

        return state;
    }

    public void Reset()
    {
        CurrentState = Initial;
        _trace.Clear();
    }

    public bool IsAccepting() => Accepting.Contains(CurrentState);

    public bool Accepts(IEnumerable<string> events)
    {
        Reset();
        Process(events);
        return IsAccepting();
    }

    public IDictionary<(string State, string Event), string> MissingTransitions()
    {
        var missing = new Dictionary<(string State, string Event), string>();
        foreach (var state in States)
        {
            foreach (var @event in Alphabet)
            {
                if (!_transitions.ContainsKey((state, @event)))
                {
                    missing[(state, @event)] = string.Empty;
                }
            }
        }

        return missing;
    }

    public bool IsComplete() => MissingTransitions().Count == 0;

    public ISet<string> ReachableStates()
    {
        var reachable = new HashSet<string>(StringComparer.Ordinal) { Initial };
        var queue = new Queue<string>();
        queue.Enqueue(Initial);
        while (queue.Count > 0)
        {
            var state = queue.Dequeue();
            foreach (var @event in Alphabet)
            {
                if (_transitions.TryGetValue((state, @event), out var target) && reachable.Add(target))
                {
                    queue.Enqueue(target);
                }
            }
        }

        return reachable;
    }
}

public sealed class NFA
{
    public const string EPSILON = "";
    private readonly Dictionary<(string State, string Event), HashSet<string>> _transitions;
    private HashSet<string> _currentStates = [];

    public NFA(
        IEnumerable<string> states,
        IEnumerable<string> alphabet,
        IDictionary<(string State, string Event), IEnumerable<string>> transitions,
        string initial,
        IEnumerable<string> accepting)
    {
        States = states.ToHashSet(StringComparer.Ordinal);
        Alphabet = alphabet.ToHashSet(StringComparer.Ordinal);
        Accepting = accepting.ToHashSet(StringComparer.Ordinal);
        Initial = initial;
        _transitions = transitions.ToDictionary(pair => pair.Key, pair => pair.Value.ToHashSet(StringComparer.Ordinal));
        Reset();
    }

    public IReadOnlySet<string> States { get; }
    public IReadOnlySet<string> Alphabet { get; }
    public IReadOnlySet<string> Accepting { get; }
    public string Initial { get; }
    public IReadOnlySet<string> CurrentStates => _currentStates;

    public void Reset()
    {
        _currentStates = EpsilonClosure([Initial]);
    }

    public HashSet<string> EpsilonClosure(IEnumerable<string> states)
    {
        var closure = states.ToHashSet(StringComparer.Ordinal);
        var stack = new Stack<string>(closure);
        while (stack.Count > 0)
        {
            var state = stack.Pop();
            if (_transitions.TryGetValue((state, EPSILON), out var targets))
            {
                foreach (var target in targets)
                {
                    if (closure.Add(target))
                    {
                        stack.Push(target);
                    }
                }
            }
        }

        return closure;
    }

    public IReadOnlySet<string> Process(string @event)
    {
        var next = new HashSet<string>(StringComparer.Ordinal);
        foreach (var state in _currentStates)
        {
            if (_transitions.TryGetValue((state, @event), out var targets))
            {
                next.UnionWith(targets);
            }
        }

        _currentStates = EpsilonClosure(next);
        return _currentStates;
    }

    public bool Accepts(IEnumerable<string> events)
    {
        Reset();
        foreach (var @event in events)
        {
            Process(@event);
        }

        return _currentStates.Overlaps(Accepting);
    }
}

public static class Minimize
{
    public static DFA Run(DFA dfa)
    {
        var reachable = dfa.ReachableStates();
        var accepting = reachable.Intersect(dfa.Accepting).ToHashSet(StringComparer.Ordinal);
        var nonAccepting = reachable.Except(accepting).ToHashSet(StringComparer.Ordinal);
        var partitions = new List<HashSet<string>>();
        if (accepting.Count > 0)
        {
            partitions.Add(accepting);
        }

        if (nonAccepting.Count > 0)
        {
            partitions.Add(nonAccepting);
        }

        while (true)
        {
            var changed = false;
            var nextPartitions = new List<HashSet<string>>();
            foreach (var group in partitions)
            {
                var splits = SplitGroup(group, dfa, partitions);
                nextPartitions.AddRange(splits);
                changed |= splits.Count > 1;
            }

            partitions = nextPartitions;
            if (!changed)
            {
                break;
            }
        }

        string Name(HashSet<string> group) => group.Count == 1 ? group.First() : "{" + string.Join(",", group.OrderBy(static s => s, StringComparer.Ordinal)) + "}";
        var stateToPartition = partitions.SelectMany((group, index) => group.Select(state => (state, index))).ToDictionary(pair => pair.state, pair => pair.index, StringComparer.Ordinal);
        var transitions = new Dictionary<(string State, string Event), string>();
        foreach (var group in partitions)
        {
            var representative = group.First();
            foreach (var @event in dfa.Alphabet)
            {
                if (dfa.Transitions.TryGetValue((representative, @event), out var target))
                {
                    transitions[(Name(group), @event)] = Name(partitions[stateToPartition[target]]);
                }
            }
        }

        var minimizedAccepting = partitions.Where(group => group.Overlaps(dfa.Accepting)).Select(Name);
        return new DFA(partitions.Select(Name), dfa.Alphabet, transitions, Name(partitions[stateToPartition[dfa.Initial]]), minimizedAccepting);
    }

    private static List<HashSet<string>> SplitGroup(HashSet<string> group, DFA dfa, IReadOnlyList<HashSet<string>> partitions)
    {
        if (group.Count <= 1)
        {
            return [group];
        }

        var stateToPartition = partitions.SelectMany((partition, index) => partition.Select(state => (state, index))).ToDictionary(pair => pair.state, pair => pair.index, StringComparer.Ordinal);
        return group.GroupBy(state =>
            string.Join("|", dfa.Alphabet.OrderBy(static e => e, StringComparer.Ordinal).Select(@event =>
                dfa.Transitions.TryGetValue((state, @event), out var target) ? stateToPartition[target].ToString() : "-1")))
            .Select(states => states.ToHashSet(StringComparer.Ordinal))
            .ToList();
    }
}

public sealed class ModalStateMachine
{
    private readonly Dictionary<string, DFA> _modes;
    private readonly Dictionary<(string Mode, string Trigger), string> _modeTransitions;
    private readonly List<ModeTransitionRecord> _modeTrace = [];

    public ModalStateMachine(IDictionary<string, DFA> modes, IDictionary<(string Mode, string Trigger), string> modeTransitions, string initialMode)
    {
        _modes = new Dictionary<string, DFA>(modes, StringComparer.Ordinal);
        _modeTransitions = new Dictionary<(string Mode, string Trigger), string>(modeTransitions);
        InitialMode = initialMode;
        CurrentMode = initialMode;
    }

    public string InitialMode { get; }
    public string CurrentMode { get; private set; }
    public DFA ActiveMachine => _modes[CurrentMode];
    public IReadOnlyList<ModeTransitionRecord> ModeTrace => _modeTrace.ToList();
    public string Process(string @event) => ActiveMachine.Process(@event);

    public string TriggerMode(string trigger)
    {
        if (!_modeTransitions.TryGetValue((CurrentMode, trigger), out var target))
        {
            throw new InvalidOperationException($"No mode transition defined for ({CurrentMode}, {trigger})");
        }

        var from = CurrentMode;
        CurrentMode = target;
        ActiveMachine.Reset();
        _modeTrace.Add(new ModeTransitionRecord(from, trigger, target));
        return target;
    }

    public void Reset()
    {
        CurrentMode = InitialMode;
        _modeTrace.Clear();
        foreach (var mode in _modes.Values)
        {
            mode.Reset();
        }
    }
}

public sealed class PushdownAutomaton
{
    private readonly Dictionary<(string State, string? Event, string StackTop), PDATransition> _index;
    private readonly HashSet<string> _accepting;
    private readonly List<string> _stack = [];
    private readonly List<PDATraceEntry> _trace = [];

    public PushdownAutomaton(
        IEnumerable<string> states,
        IEnumerable<string> inputAlphabet,
        IEnumerable<string> stackAlphabet,
        IEnumerable<PDATransition> transitions,
        string initial,
        string initialStackSymbol,
        IEnumerable<string> accepting)
    {
        Initial = initial;
        InitialStackSymbol = initialStackSymbol;
        _accepting = accepting.ToHashSet(StringComparer.Ordinal);
        _index = transitions.ToDictionary(transition => (transition.Source, transition.Event, transition.StackRead), transition => transition);
        Reset();
    }

    public string Initial { get; }
    public string InitialStackSymbol { get; }
    public string CurrentState { get; private set; } = string.Empty;
    public IReadOnlyList<string> Stack => _stack.ToList();
    public IReadOnlyList<PDATraceEntry> Trace => _trace.ToList();

    public void Reset()
    {
        CurrentState = Initial;
        _stack.Clear();
        _stack.Add(InitialStackSymbol);
        _trace.Clear();
    }

    public void Process(string @event)
    {
        Step(@event);
        while (TryStepEpsilon())
        {
        }
    }

    public void Process(IEnumerable<string> events)
    {
        foreach (var @event in events)
        {
            Process(@event);
        }

        while (TryStepEpsilon())
        {
        }
    }

    public bool Accepts(IEnumerable<string> events)
    {
        Reset();
        Process(events);
        return _accepting.Contains(CurrentState);
    }

    private void Step(string? @event)
    {
        var top = _stack.Count > 0 ? _stack[^1] : string.Empty;
        if (!_index.TryGetValue((CurrentState, @event, top), out var transition))
        {
            throw new InvalidOperationException($"No PDA transition defined for ({CurrentState}, {@event}, {top})");
        }

        ApplyTransition(transition);
    }

    private bool TryStepEpsilon()
    {
        var top = _stack.Count > 0 ? _stack[^1] : string.Empty;
        if (!_index.TryGetValue((CurrentState, null, top), out var transition))
        {
            return false;
        }

        ApplyTransition(transition);
        return true;
    }

    private void ApplyTransition(PDATransition transition)
    {
        if (_stack.Count == 0 || _stack[^1] != transition.StackRead)
        {
            throw new InvalidOperationException("PDA stack top does not match transition");
        }

        _stack.RemoveAt(_stack.Count - 1);
        foreach (var symbol in transition.StackPush)
        {
            _stack.Add(symbol);
        }

        CurrentState = transition.Target;
        _trace.Add(new PDATraceEntry(transition.Source, transition.Event, transition.StackRead, transition.Target, transition.StackPush, _stack.ToList()));
    }
}
