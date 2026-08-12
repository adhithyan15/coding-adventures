using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Mosaic.Generated;

internal sealed class TaskAppComponent
{
    public string AppTitle { get; set; } = "unset";
    public string NewTaskName { get; set; } = "unset";
}

internal sealed class NewTaskNameChangeEvent
{
    public string MosaicName => "newTaskNameChange";
    public IReadOnlyDictionary<string, object?> MosaicPayload { get; } =
        new Dictionary<string, object?> { ["value"] = "Ship on Windows" };
}

internal static class Program
{
    private static async Task Main()
    {
        MosaicRuntimeHost.LoadRequired();
        var component = new TaskAppComponent();
        try
        {
            var startStatus = MosaicRuntimeHost.ApplyRequiredProps(
                component, "app-title", "new-task-name");
            Require(startStatus == "Status: Mosaic runtime props loaded", "startup status");
            Require(component.AppTitle == "Tasks — auto-scheduled", "initial app title");
            Require(component.NewTaskName == "", "initial task composer");

            var result = await MosaicRuntimeHost.HandleRequiredEvent(
                component,
                new NewTaskNameChangeEvent(),
                "app-title",
                "new-task-name");
            Require(
                result.Status == "Status: Mosaic runtime handled newTaskNameChange",
                "dispatch status");
            Require(component.NewTaskName == "Ship on Windows", "revised task composer");
        }
        finally
        {
            MosaicRuntimeHost.Close();
        }

        Console.WriteLine("TaskApp XAML Rust runtime conformance passed");
    }

    private static void Require(bool condition, string assertion)
    {
        if (!condition) throw new InvalidOperationException($"Failed assertion: {assertion}");
    }
}
