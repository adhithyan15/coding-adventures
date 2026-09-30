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

            CheckEnvironmentReport(component, initialCount + 4);
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
    private static void CheckEnvironmentReport(ConformanceComponent component, long count)
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

        // Invalid: refused, the props stay, and it is not remembered -- sent
        // again, it is refused again rather than dropped as a duplicate.
        var invalid = new Dictionary<string, string>(compact) { ["sizeClass"] = "enormous" };
        var refusal = MosaicRuntimeHost.ReportEnvironment(component, invalid, required);
        Require(refusal?.StartsWith("Status: Mosaic runtime refused the environment", StringComparison.Ordinal) == true, "an invalid report is refused");
        Require(MosaicRuntimeHost.ReportEnvironment(component, invalid, required) is not null, "a refused report is not remembered");
        MosaicRuntimeHost.ApplyRequiredProps(component, required);
        Require(component.Count == count && component.Status == "dispatched", "a refusal keeps the props");

        // Sent or not: every dispatch persists, so with a state file the
        // file's reappearance shows whether a report reached the runtime.
        var statePath = Environment.GetEnvironmentVariable("MOSAIC_APP_STATE_PATH");
        if (string.IsNullOrWhiteSpace(statePath)) return;
        Require(File.Exists(statePath), "state persisted");
        File.Delete(statePath);
        Require(MosaicRuntimeHost.ReportEnvironment(component, compact, required) is null, "unchanged report");
        Require(!File.Exists(statePath), "an unchanged report is not sent");
        Require(MosaicRuntimeHost.ReportEnvironment(component, regular, required) is null, "changed report accepted");
        Require(File.Exists(statePath), "a changed report is sent");
    }

    private static void Require(bool condition, string assertion)
    {
        if (!condition) throw new InvalidOperationException($"Failed assertion: {assertion}");
    }
}
