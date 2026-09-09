// Drives the emitted XAML host against the conformance runtime.
//
// This crate's other tests assert on the TEXT of the emitted host. That cannot
// tell you it compiles, let alone that it behaves -- the Compose port of this
// same change shipped three missing imports past every text assertion. The
// defect this exists for is behavioural: before the host completed effects, an
// `await` was dropped and the app waited forever with nothing reporting it.
//
// One scenario per process, chosen by MOSAIC_PROBE_CASE, because the host reads
// MOSAIC_APP_STATE_PATH once at load and a shared state file would let one
// scenario restore another's count.
//
// Two names that look alike and are not: `component.Status` is the application's
// own `status` PROP, projected onto the component; `runtimeStatus` is the
// host's status STRING, which carries the persistence warning and the settle
// error. The assertions below need both, so neither is called just "status".
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Mosaic.Generated;

/// Props land here by reflection, the way a real XAML page receives them.
///
/// Every field starts at a sentinel no run produces, so a prop the update never
/// carried shows as "still the sentinel" rather than reading as zero -- which is
/// how the Qt version of this file first passed vacuously.
internal sealed class EffectComponent
{
    public long AwaitedEffects { get; set; } = -1;
    public long Count { get; set; } = long.MinValue;
    public string Status { get; set; } = "unset";
}

internal sealed class RequestEffect
{
    public RequestEffect(bool batch) => MosaicName = batch
        ? "requestEffectBatch"
        : "requestEffect";

    public string MosaicName { get; }
    public object MosaicPayload => new Dictionary<string, object?> { ["notify"] = false };
}

internal static class Driver
{
    private static int failures;

    private static void Check(bool ok, string what)
    {
        Console.WriteLine(what.PadRight(60) + (ok ? "ok" : "FAIL"));
        if (!ok) failures += 1;
    }

    private static long Awaited(EffectComponent component, string context)
    {
        if (component.AwaitedEffects == -1)
        {
            Console.WriteLine($"VACUOUS: {context} carries no awaitedEffects prop");
            failures += 1;
        }
        return component.AwaitedEffects;
    }

    private static long Counted(EffectComponent component, string context)
    {
        if (component.Count == long.MinValue)
        {
            Console.WriteLine($"VACUOUS: {context} carries no count prop");
            failures += 1;
        }
        return component.Count;
    }

    /// Whether persistence is working, as the application can see it.
    ///
    /// This host exposes no snapshot method -- it persists internally after
    /// every settle -- so the observable is the status string. The runtime
    /// refuses to snapshot while an effect is pending, `PersistSnapshot` catches
    /// that refusal, and `Status` carries it. That is the same user-visible
    /// consequence the other four hosts assert by calling `snapshot()`, reached
    /// through the API this host actually has rather than one added for a test.
    private static bool Persisted(string runtimeStatus) =>
        !runtimeStatus.Contains("Could not persist", StringComparison.Ordinal);

    private static string Suffix(bool ok, string runtimeStatus) =>
        ok ? string.Empty : $" [{runtimeStatus}]";

    private static bool IsAwait(string delivery) =>
        string.Equals(delivery, "await", StringComparison.OrdinalIgnoreCase);

    private static async Task<(EffectComponent Component, string RuntimeStatus)> Request(
        bool batch = false)
    {
        var component = new EffectComponent();
        var result = await MosaicRuntimeHost.HandleEvent(component, new RequestEffect(batch));
        return (component, result?.Status ?? string.Empty);
    }

    /// A fresh app awaits nothing, and an await nobody answers is failed.
    private static async Task CaseUnhandled()
    {
        var startup = new EffectComponent();
        MosaicRuntimeHost.ApplyProps(startup);
        Check(Awaited(startup, "startup") == 0, "a fresh app awaits nothing");

        // No handler connected -- what every native host did before completion
        // existed. The effect must come back failed, not sit pending forever.
        var (component, _) = await Request();
        Check(Awaited(component, "unhandled") == 0, "an unanswered await is failed, not dropped");
        Check(
            component.Status.Contains("no host handler", StringComparison.Ordinal),
            "the app is told why, rather than just waiting");
        Check(Counted(component, "unhandled") == 0, "a failed completion does not advance the app");
    }

    /// A handler that answers properly settles the effect and moves the app.
    private static async Task CaseAnswered()
    {
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!IsAwait(delivery)) return;
            MosaicRuntimeHost.CompleteEffect(id, new Dictionary<string, object?>
            {
                ["ok"] = new Dictionary<string, object?> { ["amount"] = 5 },
            });
        };
        var (component, _) = await Request();
        Check(Awaited(component, "handled") == 0, "an answered await is settled");
        Check(Counted(component, "handled") == 5, "the handler's value reached the app");
    }

    /// A batch where the handler answers BOTH, each chaining.
    ///
    /// One slot holding "the last update" drops the first answer's minted
    /// effect: it exists in no collection the host kept, so it is never emitted,
    /// never failed, and permanently pending -- which kills persistence.
    /// Answering a single effect cannot reach it.
    private static async Task CaseBatchBothAnswered()
    {
        var answers = 0;
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!IsAwait(delivery)) return;
            var chain = answers < 2; // chain only the first two, or this never ends
            answers += 1;
            MosaicRuntimeHost.CompleteEffect(id, new Dictionary<string, object?>
            {
                ["ok"] = new Dictionary<string, object?> { ["amount"] = 1, ["chain"] = chain },
            });
        };
        var (component, runtimeStatus) = await Request(batch: true);
        Check(answers >= 2, "the handler answered both effects of the batch");
        Check(
            Awaited(component, "both-answered batch") == 0,
            "a fully-answered chaining batch leaves nothing outstanding");
        var persisted = Persisted(runtimeStatus);
        Check(
            persisted,
            "state still persists after a fully-answered chaining batch"
                + Suffix(persisted, runtimeStatus));
    }

    /// A batch where the handler answers one and ignores the other.
    private static async Task CaseBatchPartlyAnswered()
    {
        var answeredOne = false;
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!IsAwait(delivery) || answeredOne) return;
            answeredOne = true;
            MosaicRuntimeHost.CompleteEffect(id, new Dictionary<string, object?>
            {
                ["ok"] = new Dictionary<string, object?> { ["amount"] = 3, ["chain"] = true },
            });
        };
        var (component, runtimeStatus) = await Request(batch: true);
        Check(
            Awaited(component, "mixed batch") == 0,
            "a partly-answered batch leaves nothing outstanding");
        var persisted = Persisted(runtimeStatus);
        Check(
            persisted,
            "state still persists after a partly-answered batch"
                + Suffix(persisted, runtimeStatus));
    }

    /// A handler that throws.
    ///
    /// The handler runs inside the settle loop, so an escaping exception leaves
    /// the id in `awaiting` with nothing left to discharge it -- and the runtime
    /// refuses to snapshot while anything is pending, so one throwing handler
    /// costs the process its persistence for good.
    private static async Task CaseThrowingHandler()
    {
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (IsAwait(delivery)) throw new InvalidOperationException("the dialog exploded");
        };
        var (component, runtimeStatus) = await Request();
        Check(
            Awaited(component, "throwing handler") == 0,
            "a handler that throws does not leave the effect pending");
        Check(
            component.Status.Contains("the dialog exploded", StringComparison.Ordinal),
            "the app is told the handler failed, and why");
        var persisted = Persisted(runtimeStatus);
        Check(
            persisted,
            "state still persists after a handler that threw" + Suffix(persisted, runtimeStatus));
    }

    /// A handler that answers every effect by minting another one, forever.
    ///
    /// The settle loop is bounded at 64 rounds precisely so this terminates. The
    /// bound has to both stop AND report -- and reporting is the part this host
    /// nearly lost: `Dispatch` returns void and `error` is not a prop, so the
    /// guard's reason had nowhere to go until `Status` carried it.
    private static async Task CaseRunawayChaining()
    {
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!IsAwait(delivery)) return;
            MosaicRuntimeHost.CompleteEffect(id, new Dictionary<string, object?>
            {
                ["ok"] = new Dictionary<string, object?> { ["amount"] = 0, ["chain"] = true },
            });
        };
        var component = new EffectComponent();
        var result = await MosaicRuntimeHost.HandleEvent(component, new RequestEffect(false));
        Check(result is not null, "a runaway chain returns instead of spinning forever");
        Check(
            result?.Status.Contains("did not settle", StringComparison.Ordinal) == true,
            "a runaway chain is reported rather than abandoned quietly");
    }

    /// A handler that closes the host from inside the settle.
    ///
    /// "The user shut the window while the import dialog was open." The lock is
    /// a Monitor, so this re-enters and succeeds -- and this host is the only
    /// one of the five that UNMAPS the runtime, so afterwards the settle loop
    /// would drive `CompleteEffectOnce` and `PersistSnapshot` through delegates
    /// pointing into a freed module. An AccessViolationException is uncatchable
    /// in .NET, so there is no failure to assert on: the process simply dies
    /// here. Reaching the next line at all is the assertion.
    private static async Task CaseHandlerClosesHost()
    {
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!IsAwait(delivery)) return;
            MosaicRuntimeHost.Close();
        };
        var component = new EffectComponent();
        var result = await MosaicRuntimeHost.HandleEvent(component, new RequestEffect(false));
        Check(result is not null, "a handler that closes the host does not kill the process");
        // And the host stays closed rather than half-open. It REFUSES by
        // throwing -- the nullable return of `ApplyProps` means "no runtime was
        // ever available", not "the one you had is gone" -- so the assertion is
        // that it throws, not that it quietly hands back stale props.
        var refused = false;
        try
        {
            MosaicRuntimeHost.ApplyProps(new EffectComponent());
        }
        catch (ObjectDisposedException)
        {
            refused = true;
        }
        Check(refused, "a closed host refuses further props rather than serving stale ones");
    }

    /// Take ownership, answer LATER, from another thread.
    ///
    /// The lock held across the handler is a Monitor, so answering inline is
    /// reentrant and safe -- but a real dialog answers from whatever thread
    /// finished the work, which is a different lock acquisition entirely.
    private static async Task CaseDeferred()
    {
        ulong? deferredId = null;
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!IsAwait(delivery)) return;
            deferredId = id;
            MosaicRuntimeHost.DeferEffect(id); // "the dialog is open"
        };

        var (component, runtimeStatus) = await Request();
        // Deferring something the runtime is not waiting on must be refused, or
        // the fail sweep is switched off for an effect nothing will ever answer.
        Check(
            !MosaicRuntimeHost.DeferEffect(99999UL),
            "deferring an effect nothing awaits is refused");
        Check(deferredId is not null, "the handler was offered the effect");
        Check(
            Awaited(component, "deferred") == 1,
            "a deferred effect stays outstanding rather than being failed");
        Check(
            !Persisted(runtimeStatus),
            "state cannot be persisted while a deferred effect is outstanding");

        // ...the dialog closes, on another thread, whenever.
        var answering = Task.Run(() =>
        {
            MosaicRuntimeHost.EffectHandler = null;
            MosaicRuntimeHost.CompleteEffect(deferredId!.Value, new Dictionary<string, object?>
            {
                ["ok"] = new Dictionary<string, object?> { ["amount"] = 9 },
            });
        });
        var finished = await Task.WhenAny(answering, Task.Delay(TimeSpan.FromSeconds(5)));
        Check(finished == answering, "answering from another thread does not deadlock");
        await answering;

        var settled = new EffectComponent();
        var settledStatus = MosaicRuntimeHost.ApplyProps(settled) ?? string.Empty;
        Check(Awaited(settled, "answered-late") == 0, "answering a deferred effect settles it");
        Check(Counted(settled, "answered-late") == 9, "the deferred answer's value reached the app");
        var persisted = Persisted(settledStatus);
        Check(
            persisted,
            "state persists again once the deferred effect is answered"
                + Suffix(persisted, settledStatus));
    }

    public static async Task<int> Main()
    {
        var probeCase = Environment.GetEnvironmentVariable("MOSAIC_PROBE_CASE") ?? string.Empty;
        switch (probeCase)
        {
            case "unhandled": await CaseUnhandled(); break;
            case "answered": await CaseAnswered(); break;
            case "batch-both": await CaseBatchBothAnswered(); break;
            case "batch-mixed": await CaseBatchPartlyAnswered(); break;
            case "throwing": await CaseThrowingHandler(); break;
            case "runaway": await CaseRunawayChaining(); break;
            case "closes": await CaseHandlerClosesHost(); break;
            case "deferred": await CaseDeferred(); break;
            default:
                Console.WriteLine($"unknown MOSAIC_PROBE_CASE `{probeCase}`");
                return 2;
        }
        Console.WriteLine(failures == 0 ? "case passed" : $"{failures} check(s) failed");
        return failures == 0 ? 0 : 1;
    }
}
