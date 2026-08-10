using System;
using System.Collections.Generic;
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
    private static async Task Main()
    {
        if (!MosaicRuntimeHost.IsAvailable)
            throw new InvalidOperationException("standard XAML binding did not load the Rust app");

        var component = new ConformanceComponent();
        try
        {
            var startStatus = MosaicRuntimeHost.ApplyProps(component);
            Require(startStatus == "Status: Mosaic runtime props loaded", "startup status");
            Require(component.Count == 0, "initial count");
            Require(component.Platform == "windows", "startup platform");
            Require(component.Status == "started", "startup props");

            var result = await MosaicRuntimeHost.HandleEvent(component, new IncrementEvent());
            Require(result?.Status == "Status: Mosaic runtime handled increment", "dispatch status");
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
