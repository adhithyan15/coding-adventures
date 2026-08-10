using System;
using System.Collections.Generic;
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

        var component = new ConformanceComponent();
        try
        {
            var startStatus = MosaicRuntimeHost.ApplyRequiredProps(
                component, "count", "platform", "status");
            Require(startStatus == "Status: Mosaic runtime props loaded", "startup status");
            Require(component.Count == 0, "initial count");
            Require(component.Platform == "windows", "startup platform");
            Require(component.Status == "started", "startup props");

            var result = await MosaicRuntimeHost.HandleRequiredEvent(
                component, new IncrementEvent(), "count", "platform", "status");
            Require(result.Status == "Status: Mosaic runtime handled increment", "dispatch status");
            Require(component.Count == 4, "dispatched count");
            Require(component.Status == "dispatched", "dispatched props");
        }
        finally
        {
            MosaicRuntimeHost.Close();
        }

        Console.WriteLine("Mosaic XAML Rust runtime conformance passed");
    }

    private static void Require(bool condition, string assertion)
    {
        if (!condition) throw new InvalidOperationException($"Failed assertion: {assertion}");
    }
}
