using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Mosaic.Generated;

internal sealed class TaskAppComponent
{
    public List<List<string>> TaskRows { get; set; } = [];
    public string RingPercent { get; set; } = "unset";
}

internal sealed class TaskEvent(string name, IReadOnlyDictionary<string, object?>? payload = null)
{
    public string MosaicName { get; } = name;
    public IReadOnlyDictionary<string, object?> MosaicPayload { get; } =
        payload ?? new Dictionary<string, object?>();
}

internal static class Program
{
    private const string TaskName = "Native acceptance task";
    private const string PersistedTaskName = "Persisted native task";
    private const string Due = "2026-01-09";
    private const string Schedule = "2026-01-05 → 2026-01-05";

    private static async Task Main()
    {
        var restoredOnLaunch = Environment.GetEnvironmentVariable("MOSAIC_EXPECT_RESTORED") == "1";
        MosaicRuntimeHost.LoadRequired();
        var component = new TaskAppComponent();
        try
        {
            MosaicRuntimeHost.ApplyRequiredProps(component, "task-rows", "ring-percent");
            if (restoredOnLaunch)
            {
                RequireTask(component.TaskRows, PersistedTaskName);
                await Dispatch(component, "onDeleteTask", ("index", 0));
                Require(component.TaskRows.Count == 0, "delete restored task");
                Console.WriteLine("TaskApp XAML persisted-restart conformance passed");
                return;
            }

            Require(component.TaskRows.Count == 0, "fresh task list");
            var rejected = false;
            try
            {
                await Dispatch(component, "onNewTaskNameChange", ("value", 7));
            }
            catch (MosaicRuntimeException)
            {
                rejected = true;
            }
            Require(rejected, "invalid input rejected");
            Require(component.TaskRows.Count == 0, "invalid input preserved state");

            await Dispatch(component, "onNewTaskNameChange", ("value", TaskName));
            await Dispatch(component, "onNewTaskDueChange", ("value", Due));
            await Dispatch(component, "onAddTask");
            Require(component.TaskRows.Single()[3] == "", "Board mode hides schedule");
            await Dispatch(component, "onToggleProjectComplexity");
            RequireTask(component.TaskRows, TaskName);

            await Dispatch(component, "onToggleTask", ("index", 0));
            Require(component.TaskRows.Single()[0] == "✓", "complete task");
            Require(component.RingPercent == "100%", "completion projection");
            await Dispatch(component, "onToggleTask", ("index", 0));
            Require(component.TaskRows.Single()[0] == "○", "reopen task");
            await Dispatch(component, "onDeleteTask", ("index", 0));
            Require(component.TaskRows.Count == 0, "delete task");

            await Dispatch(component, "onNewTaskNameChange", ("value", PersistedTaskName));
            await Dispatch(component, "onNewTaskDueChange", ("value", Due));
            await Dispatch(component, "onAddTask");
            RequireTask(component.TaskRows, PersistedTaskName);
        }
        finally
        {
            MosaicRuntimeHost.Close();
        }

        Console.WriteLine("TaskApp XAML native lifecycle conformance passed");
    }

    private static async Task Dispatch(
        TaskAppComponent component,
        string name,
        params (string Name, object? Value)[] fields)
    {
        var payload = fields.ToDictionary(field => field.Name, field => field.Value);
        var result = await MosaicRuntimeHost.HandleRequiredEvent(
            component,
            new TaskEvent(name, payload),
            "task-rows",
            "ring-percent");
        Require(
            result.Status.Contains($"Mosaic runtime handled {name}", StringComparison.Ordinal),
            $"{name} dispatch status");
    }

    private static void RequireTask(List<List<string>> rows, string name)
    {
        Require(rows.Count == 1, "one task row");
        var row = rows.Single();
        Require(row.Count >= 4, "task row projection width");
        Require(row[1] == name, "task name projection");
        Require(row[2] == $"due {Due}", "task due projection");
        Require(row[3] == Schedule, "Rust schedule start/finish projection");
    }

    private static void Require(bool condition, string assertion)
    {
        if (!condition) throw new InvalidOperationException($"Failed assertion: {assertion}");
    }
}
