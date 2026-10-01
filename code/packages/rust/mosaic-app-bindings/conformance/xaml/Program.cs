using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace Mosaic.Generated;

internal sealed class ConformanceComponent
{
    public long Count { get; set; } = -1;
    public string Platform { get; set; } = "unset";
    public string Status { get; set; } = "unset";
}

internal sealed class IncrementEvent
{
    public string MosaicName => "increment";
    public IReadOnlyDictionary<string, object?> MosaicPayload { get; } =
        new Dictionary<string, object?> { ["amount"] = 4L };
}

/// <summary>
/// Switches the conformance app's environment failure (UI48 §7.7): while on,
/// every <c>environmentChanged</c> that reaches the app is an app error.
/// </summary>
internal sealed class FailEnvironmentEvent(bool fail)
{
    public string MosaicName => "failEnvironment";
    public IReadOnlyDictionary<string, object?> MosaicPayload { get; } =
        new Dictionary<string, object?> { ["fail"] = fail };
}

internal static class Program
{
    private static async Task Main(string[] args)
    {
        if (args.Contains("--expect-required-failure"))
        {
            try
            {
                MosaicRuntimeHost.LoadRequired();
                throw new InvalidOperationException("required XAML binding unexpectedly loaded Rust");
            }
            catch (InvalidOperationException error) when (
                error.Message.Contains(
                    "native-complete requires the Mosaic Rust application runtime",
                    StringComparison.Ordinal))
            {
                Console.WriteLine("Mosaic XAML required-runtime failure passed");
                return;
            }
        }

        if (args.Contains("--expect-missing-prop-failure"))
        {
            MosaicRuntimeHost.LoadRequired();
            try
            {
                MosaicRuntimeHost.ApplyRequiredProps(
                    new ConformanceComponent(), "missing-required-prop");
                throw new InvalidOperationException(
                    "required XAML binding accepted a missing required prop");
            }
            catch (InvalidOperationException error) when (
                error.Message.Contains(
                    "Mosaic runtime props are missing required value 'missing-required-prop'",
                    StringComparison.Ordinal))
            {
                Console.WriteLine("Mosaic XAML required-prop failure passed");
                return;
            }
            finally
            {
                MosaicRuntimeHost.Close();
            }
        }

        MosaicRuntimeHost.LoadRequired();

        var restoredOnLaunch = Environment.GetEnvironmentVariable("MOSAIC_EXPECT_RESTORED") == "1";
        var expectWarning =
            Environment.GetEnvironmentVariable("MOSAIC_EXPECT_PERSISTENCE_WARNING") == "1";
        var initialCount = restoredOnLaunch ? 4L : 0L;
        var component = new ConformanceComponent();
        try
        {
            var startStatus = MosaicRuntimeHost.ApplyRequiredProps(
                component, "count", "platform", "status");
            Require(
                startStatus.StartsWith("Status: Mosaic runtime props loaded", StringComparison.Ordinal),
                "startup status");
            Require(
                (startStatus != "Status: Mosaic runtime props loaded") == expectWarning,
                "startup persistence warning");
            Require(component.Count == initialCount, "initial count");
            Require(component.Platform == "windows", "startup platform");
            Require(
                component.Status == (restoredOnLaunch ? "restored" : "started"),
                "startup props");

            var result = await MosaicRuntimeHost.HandleRequiredEvent(
                component, new IncrementEvent(), "count", "platform", "status");
            Require(result.Status == "Status: Mosaic runtime handled increment", "dispatch status");
            Require(component.Count == initialCount + 4, "dispatched count");
            Require(component.Status == "dispatched", "dispatched props");

            await CheckEnvironmentReport(component, initialCount + 4);
        }
        finally
        {
            MosaicRuntimeHost.Close();
        }

        Console.WriteLine("Mosaic XAML Rust runtime conformance passed");
    }

    /// <summary>
    /// UI48 ENV4 against the conformance app, which does not react to
    /// <c>environmentChanged</c>: the runtime answers at the revision showing
    /// with no props.
    /// </summary>
    private static async Task CheckEnvironmentReport(ConformanceComponent component, long count)
    {
        string[] required = { "count", "platform", "status" };

        // The six values and the 600 / 1024 thresholds.
        var compact = MosaicRuntimeHost.EnvironmentReport(599, 800, dark: false);
        Require(compact.Count == 6, "report has the six UI48 values");
        Require(compact["sizeClass"] == "compact", "599 wide is compact");
        Require(compact["orientation"] == "portrait", "taller than wide is portrait");
        Require(compact["colorScheme"] == "light", "light scheme");
        Require(compact["pointer"] == "fine" && compact["hover"] == "hover", "desktop pointer");
        Require(compact["reducedMotion"] == "no-preference", "reduced motion default");
        var regular = MosaicRuntimeHost.EnvironmentReport(600, 600, dark: true);
        Require(regular["sizeClass"] == "regular", "600 wide is regular");
        Require(regular["orientation"] == "landscape", "a square is landscape");
        Require(regular["colorScheme"] == "dark", "dark scheme");
        Require(MosaicRuntimeHost.EnvironmentReport(1023, 700, false)["sizeClass"] == "regular", "1023 wide is regular");
        Require(MosaicRuntimeHost.EnvironmentReport(1024, 700, false)["sizeClass"] == "expanded", "1024 wide is expanded");

        // Ignored: nothing is re-applied (the sentinel survives), and the
        // props showing are kept for the next strict apply.
        component.Status = "sentinel";
        Require(MosaicRuntimeHost.ReportEnvironment(component, compact, required) is null, "ignored report accepted");
        Require(component.Status == "sentinel", "an ignored report re-applies nothing");
        MosaicRuntimeHost.ApplyRequiredProps(component, required);
        Require(component.Count == count && component.Status == "dispatched", "an ignored report keeps the props");

        // Invalid: refused, and the props stay.
        var invalid = new Dictionary<string, string>(compact) { ["sizeClass"] = "enormous" };
        var refusal = MosaicRuntimeHost.ReportEnvironment(component, invalid, required);
        Require(refusal?.StartsWith("Status: Mosaic environment report failed", StringComparison.Ordinal) == true, "an invalid report is refused");
        MosaicRuntimeHost.ApplyRequiredProps(component, required);
        Require(component.Count == count && component.Status == "dispatched", "a refusal keeps the props");
        // The identical refused report is held back, not refused again.
        Require(MosaicRuntimeHost.ReportEnvironment(component, invalid, required) is null, "a refused report is not re-sent");

        // Sent or not. While failEnvironment is on, every report that reaches
        // the app is an app error -- a failure that is not the report's
        // fault -- so a failure status proves a report was sent and null that
        // it was held back.
        async Task FailEnvironment(bool fail, string assertion)
        {
            var result = await MosaicRuntimeHost.HandleRequiredEvent(
                component, new FailEnvironmentEvent(fail), required);
            Require(result.Status.StartsWith("Status: Mosaic runtime handled failEnvironment", StringComparison.Ordinal), assertion);
        }
        bool Failed(string? status) =>
            status?.StartsWith("Status: Mosaic environment report failed", StringComparison.Ordinal) == true;
        await FailEnvironment(true, "the app is told to fail environment changes");
        Require(MosaicRuntimeHost.ReportEnvironment(component, invalid, required) is null, "the refused report is not re-sent");
        // The refusal did not replace the last report taken: that one is
        // still unchanged, so still not sent.
        Require(MosaicRuntimeHost.ReportEnvironment(component, compact, required) is null, "an unchanged report is not sent");
        var failure = MosaicRuntimeHost.ReportEnvironment(component, regular, required);
        Require(Failed(failure) && failure!.Contains("Mosaic application error", StringComparison.Ordinal), "a changed report is sent, and the app's failure reported");
        Require(Failed(MosaicRuntimeHost.ReportEnvironment(component, regular, required)), "a report that failed transiently is sent again");
        await FailEnvironment(false, "the app is told to take environment changes again");

        // An ignored report writes no state: nothing the app saves changed.
        var statePath = Environment.GetEnvironmentVariable("MOSAIC_APP_STATE_PATH");
        if (string.IsNullOrWhiteSpace(statePath))
        {
            Console.WriteLine("MOSAIC_APP_STATE_PATH unset: skipped the checks that an ignored report writes no state");
            return;
        }
        Require(File.Exists(statePath), "state persisted");
        File.Delete(statePath);
        component.Status = "sentinel";
        Require(MosaicRuntimeHost.ReportEnvironment(component, regular, required) is null, "once the failure passes, the same report is taken");
        Require(component.Status == "sentinel", "an ignored changed report re-applies nothing");
        Require(!File.Exists(statePath), "an ignored report does not rewrite the state file");
        // With the state path a directory, any write fails and says so. With
        // no failed save pending, an ignored report attempts none, so it
        // raises no warning; an event still does, in its own status. Once a
        // save has failed, the next ignored report retries it -- so a kill
        // before the next event does not lose that revision -- and reports
        // the warning's clearing in a status, as an event would.
        const string loaded = "Status: Mosaic runtime props loaded";
        Directory.CreateDirectory(statePath);
        Require(MosaicRuntimeHost.ReportEnvironment(component, compact, required) is null, "a changed report accepted");
        Require(MosaicRuntimeHost.ApplyRequiredProps(component, required) == loaded, "an ignored report raises no persistence warning");
        var warned = await MosaicRuntimeHost.HandleRequiredEvent(component, new FailEnvironmentEvent(false), required);
        Require(warned.Status.Contains("Could not persist Mosaic state", StringComparison.Ordinal), "an event that cannot persist surfaces the warning at once");
        Require(MosaicRuntimeHost.ReportEnvironment(component, regular, required) is null, "an ignored report while saving still fails changes nothing shown");
        Require(MosaicRuntimeHost.ApplyRequiredProps(component, required).Contains("Could not persist Mosaic state", StringComparison.Ordinal), "the warning stays while saving fails");
        Directory.Delete(statePath);
        Require(MosaicRuntimeHost.ReportEnvironment(component, compact, required) == "Status: Mosaic runtime handled environmentChanged", "an ignored report retries a failed save and reports the warning cleared");
        Require(File.Exists(statePath), "the retried save wrote the state file");
        Require(MosaicRuntimeHost.ApplyRequiredProps(component, required) == loaded, "the cleared warning stays cleared");
        await FailEnvironment(true, "the app is told to fail environment changes once more");
        Require(MosaicRuntimeHost.ReportEnvironment(component, compact, required) is null, "the report that retried the save is held back");
        await FailEnvironment(false, "the app is left taking environment changes");
    }

    private static void Require(bool condition, string assertion)
    {
        if (!condition) throw new InvalidOperationException($"Failed assertion: {assertion}");
    }
}
