defmodule BuildTool.Reporter do
  @moduledoc """
  Formats and prints a summary table of build results.

  ## Output format

  The report is designed for terminal display — a fixed-width table with
  aligned columns, followed by a summary line:

      Build Report
      ============
      Package                    Status     Duration
      python/logic-gates         SKIPPED    -
      python/arithmetic          BUILT      2.3s
      python/arm-simulator       FAILED     0.5s
      python/riscv-simulator     DEP-SKIP   - (dep failed)

      Total: 21 packages | 5 built | 14 skipped | 1 failed | 1 dep-skipped

  The report is sorted by package name for consistent output across runs.
  Status names are uppercased for visual prominence.

  ## Design note: pure function + side-effecting wrapper

  `format_report/1` is a pure function that returns a string — easy to test,
  easy to compose. `print_report/1` wraps it with `IO.puts` for convenience.
  This separation follows the "functional core, imperative shell" pattern.
  """

  # ---------------------------------------------------------------------------
  # Status display names
  # ---------------------------------------------------------------------------
  #
  # We uppercase status names for visual clarity in the terminal. The mapping
  # also serves as documentation of all possible statuses.

  @status_display %{
    "built" => "BUILT",
    "failed" => "FAILED",
    "skipped" => "SKIPPED",
    "dep-skipped" => "DEP-SKIP",
    "would-build" => "WOULD-BUILD"
  }

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Produces the build report as a string.

  This is the pure function — it doesn't print anything, making it easy
  to test. The report includes:

    1. A header ("Build Report")
    2. A fixed-width table with Package, Status, and Duration columns
    3. Error details for any failed packages
    4. A summary line with counts of each status

  ## Parameters

    - `results` — map from package name to result map

  ## Example

      iex> results = %{
      ...>   "python/logic-gates" => %{status: "built", duration: 2.3, stderr: "", stdout: ""},
      ...>   "go/graph" => %{status: "skipped", duration: 0.0, stderr: "", stdout: ""}
      ...> }
      iex> report = BuildTool.Reporter.format_report(results)
      iex> String.contains?(report, "BUILT")
      true
  """
  def format_report(results) do
    buf = ["\nBuild Report\n", "============\n"]

    if map_size(results) == 0 do
      Enum.join(buf ++ ["No packages processed.\n"])
    else
      # Calculate the maximum package name length for column alignment.
      max_name_len =
        results
        |> Map.keys()
        |> Enum.map(&String.length/1)
        |> Enum.max()
        |> max(String.length("Package"))

      # Header row.
      header =
        String.pad_trailing("Package", max_name_len) <>
          "   " <> String.pad_trailing("Status", 12) <> " Duration\n"

      # Sort results by package name for consistent output.
      names = results |> Map.keys() |> Enum.sort()

      # Data rows.
      rows =
        Enum.map(names, fn name ->
          result = results[name]
          status = Map.get(@status_display, result.status, String.upcase(result.status))
          duration = format_duration(result.duration, result.status)

          String.pad_trailing(name, max_name_len) <>
            "   " <> String.pad_trailing(status, 12) <> " " <> duration <> "\n"
        end)

      # Show error details for failed packages.
      error_details =
        names
        |> Enum.filter(fn name ->
          result = results[name]
          result.status == "failed" and (result.stderr != "" or result.stdout != "")
        end)
        |> Enum.map(fn name ->
          result = results[name]
          detail = "\n--- FAILED: #{name} ---\n"

          stderr_part =
            if result.stderr != "" do
              result.stderr <> if(String.ends_with?(result.stderr, "\n"), do: "", else: "\n")
            else
              ""
            end

          stdout_part =
            if result.stdout != "" do
              result.stdout <> if(String.ends_with?(result.stdout, "\n"), do: "", else: "\n")
            else
              ""
            end

          detail <> stderr_part <> stdout_part
        end)

      # Summary line — counts of each status.
      total = map_size(results)
      built = count_status(results, "built")
      skipped = count_status(results, "skipped")
      failed = count_status(results, "failed")
      dep_skipped = count_status(results, "dep-skipped")
      would_build = count_status(results, "would-build")

      summary = "\nTotal: #{total} packages"
      summary = if built > 0, do: summary <> " | #{built} built", else: summary
      summary = if skipped > 0, do: summary <> " | #{skipped} skipped", else: summary
      summary = if failed > 0, do: summary <> " | #{failed} failed", else: summary
      summary = if dep_skipped > 0, do: summary <> " | #{dep_skipped} dep-skipped", else: summary

      summary =
        if would_build > 0, do: summary <> " | #{would_build} would-build", else: summary

      summary = summary <> "\n"

      Enum.join(buf ++ [header] ++ rows ++ error_details ++ [summary])
    end
  end

  @doc """
  Prints the build report to stdout.

  This is the side-effecting wrapper around `format_report/1`.
  """
  def print_report(results) do
    report = format_report(results)
    IO.write(report)
  end

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Formats a duration in seconds for display.
  # Returns "-" for negligible durations, otherwise "X.Ys".
  defp format_duration(seconds, status) do
    cond do
      status == "dep-skipped" -> "- (dep failed)"
      seconds < 0.01 -> "-"
      true -> :erlang.float_to_binary(seconds, decimals: 1) <> "s"
    end
  end

  defp count_status(results, status) do
    results |> Map.values() |> Enum.count(fn r -> r.status == status end)
  end
end
