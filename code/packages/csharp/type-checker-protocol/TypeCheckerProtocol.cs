using System.Text.RegularExpressions;

namespace CodingAdventures.TypeCheckerProtocol;

/// <summary>
/// A single type-checking diagnostic with a source location.
/// </summary>
public sealed record TypeErrorDiagnostic(string Message, int Line, int Column);

/// <summary>
/// The result of a type-checking pass.
/// </summary>
public sealed class TypeCheckResult<TAst>
{
    /// <summary>Create a result with an AST and optional diagnostics.</summary>
    public TypeCheckResult(TAst typedAst, IEnumerable<TypeErrorDiagnostic>? errors = null)
    {
        TypedAst = typedAst;
        Errors = (errors ?? []).ToArray();
    }

    /// <summary>The typed or partially typed AST.</summary>
    public TAst TypedAst { get; }

    /// <summary>Diagnostics collected during checking.</summary>
    public IReadOnlyList<TypeErrorDiagnostic> Errors { get; }

    /// <summary>True when no diagnostics were reported.</summary>
    public bool Ok => Errors.Count == 0;
}

/// <summary>
/// Generic contract for language-specific type checkers.
/// </summary>
public interface ITypeChecker<TAstIn, TAstOut>
{
    /// <summary>Type-check an AST and return its typed form plus diagnostics.</summary>
    TypeCheckResult<TAstOut> Check(TAstIn ast);
}

/// <summary>
/// Marker value a hook may return to ask dispatch to keep searching.
/// </summary>
public sealed class NotHandled
{
    private NotHandled()
    {
    }

    /// <summary>The singleton marker value.</summary>
    public static NotHandled Value { get; } = new();
}

/// <summary>
/// Reusable base class for AST-driven type checkers.
/// </summary>
public abstract class GenericTypeChecker<TAst> : ITypeChecker<TAst, TAst>
{
    private readonly List<TypeErrorDiagnostic> _errors = [];
    private readonly Dictionary<(string Phase, string Kind), List<Func<TAst, object?[], object?>>> _hooks = [];

    /// <summary>Run the checker and return the AST plus collected diagnostics.</summary>
    public TypeCheckResult<TAst> Check(TAst ast)
    {
        _errors.Clear();
        Run(ast);
        return new TypeCheckResult<TAst>(ast, _errors);
    }

    /// <summary>Register a hook for a phase and normalized node kind.</summary>
    public void RegisterHook(string phase, string kind, Func<TAst, object?[], object?> hook)
    {
        ArgumentNullException.ThrowIfNull(phase);
        ArgumentNullException.ThrowIfNull(kind);
        ArgumentNullException.ThrowIfNull(hook);

        var key = (phase, NormalizeKind(kind));
        if (!_hooks.TryGetValue(key, out var hooks))
        {
            hooks = [];
            _hooks[key] = hooks;
        }

        hooks.Add(hook);
    }

    /// <summary>Dispatch a node to the first matching hook.</summary>
    public object? Dispatch(string phase, TAst node, object?[]? args = null, object? defaultValue = null)
    {
        ArgumentNullException.ThrowIfNull(phase);
        var normalized = NormalizeKind(NodeKind(node));
        foreach (var key in new[] { (phase, normalized), (phase, "*") })
        {
            if (!_hooks.TryGetValue(key, out var hooks))
            {
                continue;
            }

            foreach (var hook in hooks)
            {
                var result = hook(node, args ?? []);
                if (!ReferenceEquals(result, NotHandled.Value))
                {
                    return result;
                }
            }
        }

        return defaultValue;
    }

    /// <summary>Normalize a node kind for hook lookup.</summary>
    public static string NormalizeKind(string? kind)
    {
        if (string.IsNullOrEmpty(kind))
        {
            return string.Empty;
        }

        return Regex.Replace(kind, @"\W+", "_").Trim('_');
    }

    /// <summary>Run concrete type-checking logic.</summary>
    protected abstract void Run(TAst ast);

    /// <summary>Return a language-specific kind label for a node.</summary>
    protected abstract string? NodeKind(TAst node);

    /// <summary>Return the diagnostic location for a subject.</summary>
    protected virtual (int Line, int Column) Locate(object subject) => (1, 1);

    /// <summary>Append one diagnostic for a subject.</summary>
    protected void Error(string message, object subject)
    {
        var (line, column) = Locate(subject);
        _errors.Add(new TypeErrorDiagnostic(message, line, column));
    }
}

/// <summary>
/// Package metadata.
/// </summary>
public static class TypeCheckerProtocolPackage
{
    /// <summary>The package version.</summary>
    public const string Version = "0.1.0";
}
