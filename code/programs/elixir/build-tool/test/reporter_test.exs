defmodule BuildTool.ReporterTest do
  use ExUnit.Case, async: true

  alias BuildTool.Reporter

  # ---------------------------------------------------------------------------
  # format_report/1
  # ---------------------------------------------------------------------------

  describe "format_report/1" do
    test "formats empty results" do
      report = Reporter.format_report(%{})
      assert String.contains?(report, "Build Report")
      assert String.contains?(report, "No packages processed.")
    end

    test "formats a single built result" do
      results = %{
        "python/logic-gates" => %{
          status: "built",
          duration: 2.345,
          stdout: "",
          stderr: ""
        }
      }

      report = Reporter.format_report(results)
      assert String.contains?(report, "Build Report")
      assert String.contains?(report, "python/logic-gates")
      assert String.contains?(report, "BUILT")
      assert String.contains?(report, "2.3s")
      assert String.contains?(report, "Total: 1 packages | 1 built")
    end

    test "formats skipped result with dash for duration" do
      results = %{
        "go/graph" => %{status: "skipped", duration: 0.0, stdout: "", stderr: ""}
      }

      report = Reporter.format_report(results)
      assert String.contains?(report, "SKIPPED")
      assert String.contains?(report, "1 skipped")
    end

    test "formats dep-skipped result" do
      results = %{
        "python/sim" => %{status: "dep-skipped", duration: 0.0, stdout: "", stderr: ""}
      }

      report = Reporter.format_report(results)
      assert String.contains?(report, "DEP-SKIP")
      assert String.contains?(report, "- (dep failed)")
      assert String.contains?(report, "1 dep-skipped")
    end

    test "formats would-build result" do
      results = %{
        "ruby/lib" => %{status: "would-build", duration: 0.0, stdout: "", stderr: ""}
      }

      report = Reporter.format_report(results)
      assert String.contains?(report, "WOULD-BUILD")
      assert String.contains?(report, "1 would-build")
    end

    test "formats failed result with error details" do
      results = %{
        "python/broken" => %{
          status: "failed",
          duration: 0.5,
          stdout: "some output",
          stderr: "error: test failed"
        }
      }

      report = Reporter.format_report(results)
      assert String.contains?(report, "FAILED")
      assert String.contains?(report, "--- FAILED: python/broken ---")
      assert String.contains?(report, "error: test failed")
      assert String.contains?(report, "some output")
      assert String.contains?(report, "1 failed")
    end

    test "sorts results by package name" do
      results = %{
        "z/last" => %{status: "built", duration: 1.0, stdout: "", stderr: ""},
        "a/first" => %{status: "built", duration: 1.0, stdout: "", stderr: ""},
        "m/middle" => %{status: "skipped", duration: 0.0, stdout: "", stderr: ""}
      }

      report = Reporter.format_report(results)
      lines = String.split(report, "\n")
      # Find the data lines (after the header).
      data_lines =
        lines
        |> Enum.filter(fn l -> String.contains?(l, "BUILT") or String.contains?(l, "SKIPPED") end)

      names = Enum.map(data_lines, fn l -> l |> String.trim() |> String.split(~r/\s+/) |> hd() end)
      assert names == ["a/first", "m/middle", "z/last"]
    end

    test "includes multiple status counts in summary" do
      results = %{
        "a/built" => %{status: "built", duration: 1.0, stdout: "", stderr: ""},
        "b/skip" => %{status: "skipped", duration: 0.0, stdout: "", stderr: ""},
        "c/fail" => %{status: "failed", duration: 0.5, stdout: "", stderr: "err"},
        "d/dep" => %{status: "dep-skipped", duration: 0.0, stdout: "", stderr: ""}
      }

      report = Reporter.format_report(results)
      assert String.contains?(report, "Total: 4 packages")
      assert String.contains?(report, "1 built")
      assert String.contains?(report, "1 skipped")
      assert String.contains?(report, "1 failed")
      assert String.contains?(report, "1 dep-skipped")
    end
  end
end
