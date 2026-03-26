defmodule CodingAdventures.ProgressBar.TrackerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ProgressBar.Tracker

  # ---------------------------------------------------------------------------
  # Helper: capture output from a tracker
  # ---------------------------------------------------------------------------
  #
  # We use a StringIO process as the writer device. This lets us capture
  # everything the tracker writes without touching real stderr. After
  # stopping the tracker, we read back the accumulated output.
  #
  # This is the Elixir equivalent of Go's bytes.Buffer -- an in-memory
  # IO device that collects writes for later inspection.

  defp run_tracker(total, label \\ "", events) do
    {:ok, writer} = StringIO.open("")

    {:ok, tracker} =
      Tracker.start_link(total: total, writer: writer, label: label)

    for event <- events do
      case event do
        {:started, name} ->
          Tracker.send_event(tracker, :started, name)

        {:finished, name, status} ->
          Tracker.send_event(tracker, :finished, name, status)

        {:finished, name} ->
          Tracker.send_event(tracker, :finished, name)

        {:skipped, name} ->
          Tracker.send_event(tracker, :skipped, name)
      end
    end

    # Small sleep to let the GenServer process all cast messages.
    Process.sleep(50)
    Tracker.stop(tracker)

    {_input, output} = StringIO.contents(writer)
    StringIO.close(writer)
    output
  end

  # ---------------------------------------------------------------------------
  # Tests for event counting and basic rendering
  # ---------------------------------------------------------------------------

  describe "event counting" do
    test "empty tracker shows 0/N with waiting..." do
      output = run_tracker(5, [])
      assert output =~ "0/5"
      assert output =~ "waiting..."
    end

    test "started event adds name to building list without incrementing completed" do
      output = run_tracker(5, [started: "pkg-a"])
      assert output =~ "0/5"
      assert output =~ "pkg-a"
    end

    test "finished event increments completed and removes from building" do
      output =
        run_tracker(1, [
          {:started, "pkg-a"},
          {:finished, "pkg-a", "built"}
        ])

      assert output =~ "1/1"
      assert output =~ "done"
    end

    test "skipped event increments completed without going through building" do
      output = run_tracker(3, [{:skipped, "pkg-b"}])
      assert output =~ "1/3"
    end

    test "mixed events produce correct final count" do
      output =
        run_tracker(3, [
          {:skipped, "pkg-a"},
          {:skipped, "pkg-b"},
          {:started, "pkg-c"},
          {:finished, "pkg-c", "built"}
        ])

      assert output =~ "3/3"
      assert output =~ "done"
    end

    test "multiple started then finished events track correctly" do
      output =
        run_tracker(2, [
          {:started, "pkg-a"},
          {:started, "pkg-b"},
          {:finished, "pkg-a", "ok"},
          {:finished, "pkg-b", "ok"}
        ])

      assert output =~ "2/2"
      assert output =~ "done"
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for bar rendering
  # ---------------------------------------------------------------------------

  describe "bar rendering" do
    test "bar contains filled and empty block characters at partial completion" do
      output =
        run_tracker(4, [
          {:skipped, "a"},
          {:skipped, "b"}
        ])

      # 2/4 = 50% -> 10 filled, 10 empty
      assert output =~ "\u2588"
      assert output =~ "\u2591"
    end

    test "fully completed bar is all filled blocks" do
      output = run_tracker(1, [{:skipped, "a"}])
      full_bar = String.duplicate("\u2588", 20)
      assert output =~ full_bar
    end

    test "empty bar is all empty blocks" do
      output = run_tracker(5, [])
      empty_bar = String.duplicate("\u2591", 20)
      assert output =~ empty_bar
    end

    test "bar uses carriage return for overwriting" do
      output = run_tracker(1, [{:skipped, "a"}])
      assert output =~ "\r"
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for name truncation
  # ---------------------------------------------------------------------------

  describe "name truncation" do
    test "shows up to 3 names sorted alphabetically" do
      output =
        run_tracker(10, [
          {:started, "delta"},
          {:started, "alpha"},
          {:started, "charlie"},
          {:started, "bravo"},
          {:started, "echo"}
        ])

      assert output =~ "alpha"
      assert output =~ "bravo"
      assert output =~ "charlie"
      assert output =~ "+2 more"
    end

    test "exactly 3 names show without truncation" do
      output =
        run_tracker(10, [
          {:started, "a"},
          {:started, "b"},
          {:started, "c"}
        ])

      assert output =~ "Building: a, b, c"
      refute output =~ "more"
    end

    test "single name shows without extra formatting" do
      output = run_tracker(10, [{:started, "alpha"}])
      assert output =~ "Building: alpha"
      refute output =~ "more"
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for elapsed time
  # ---------------------------------------------------------------------------

  describe "elapsed time" do
    test "output contains elapsed time with 's)' suffix" do
      output = run_tracker(1, [])
      assert output =~ "s)"
    end

    test "output contains opening parenthesis for elapsed time" do
      output = run_tracker(1, [])
      assert output =~ "("
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for labeled (flat) mode
  # ---------------------------------------------------------------------------

  describe "labeled mode" do
    test "label appears in output" do
      output = run_tracker(3, "Level", [{:skipped, "a"}])
      assert output =~ "Level"
      assert output =~ "1/3"
    end

    test "unlabeled mode omits label prefix" do
      output = run_tracker(3, "", [{:skipped, "a"}])
      # Should start with \r[ not \r<label>
      assert output =~ "\r["
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for hierarchical progress
  # ---------------------------------------------------------------------------

  describe "hierarchical progress" do
    test "child tracker shows parent label in output" do
      {:ok, writer} = StringIO.open("")

      {:ok, parent} =
        Tracker.start_link(total: 3, writer: writer, label: "Level")

      {:ok, child} = Tracker.child(parent, 2, "Package")
      Tracker.send_event(child, :started, "pkg-a")
      Tracker.send_event(child, :finished, "pkg-a", "built")
      Tracker.send_event(child, :skipped, "pkg-b")
      Process.sleep(50)
      Tracker.finish(child)
      Process.sleep(50)
      Tracker.stop(parent)

      {_input, output} = StringIO.contents(writer)
      StringIO.close(writer)

      assert output =~ "Level"
      assert output =~ "pkg-a"
    end

    test "finishing a child advances the parent's completed count" do
      {:ok, writer} = StringIO.open("")

      {:ok, parent} =
        Tracker.start_link(total: 2, writer: writer, label: "Level")

      {:ok, child1} = Tracker.child(parent, 1, "Pkg")
      Tracker.send_event(child1, :skipped, "a")
      Process.sleep(20)
      Tracker.finish(child1)
      Process.sleep(20)

      {:ok, child2} = Tracker.child(parent, 1, "Pkg")
      Tracker.send_event(child2, :skipped, "b")
      Process.sleep(20)
      Tracker.finish(child2)
      Process.sleep(20)

      Tracker.stop(parent)

      {_input, output} = StringIO.contents(writer)
      StringIO.close(writer)

      assert output =~ "2/2"
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for concurrent sends
  # ---------------------------------------------------------------------------

  describe "concurrent sends" do
    test "many tasks can send events simultaneously without crashes" do
      {:ok, writer} = StringIO.open("")

      {:ok, tracker} =
        Tracker.start_link(total: 100, writer: writer)

      tasks =
        for i <- 1..100 do
          Task.async(fn ->
            name = "item-#{i}"
            Tracker.send_event(tracker, :started, name)
            Tracker.send_event(tracker, :finished, name, "ok")
          end)
        end

      Task.await_many(tasks, 5000)
      Process.sleep(100)
      Tracker.stop(tracker)

      {_input, output} = StringIO.contents(writer)
      StringIO.close(writer)

      assert output =~ "100/100"
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for nil safety
  # ---------------------------------------------------------------------------

  describe "nil safety" do
    test "send_event with nil pid is a no-op" do
      assert :ok == Tracker.send_event(nil, :started, "test")
    end

    test "send_event with nil pid and status is a no-op" do
      assert :ok == Tracker.send_event(nil, :finished, "test", "built")
    end

    test "child with nil parent returns nil" do
      assert nil == Tracker.child(nil, 5, "test")
    end

    test "finish with nil pid is a no-op" do
      assert :ok == Tracker.finish(nil)
    end

    test "stop with nil pid is a no-op" do
      assert :ok == Tracker.stop(nil)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for the public API module
  # ---------------------------------------------------------------------------

  describe "CodingAdventures.ProgressBar public API" do
    alias CodingAdventures.ProgressBar

    test "start_link and stop work through the public module" do
      {:ok, writer} = StringIO.open("")
      {:ok, tracker} = ProgressBar.start_link(total: 5, writer: writer)
      assert is_pid(tracker)
      ProgressBar.stop(tracker)
    end

    test "send_event delegates correctly" do
      {:ok, writer} = StringIO.open("")
      {:ok, tracker} = ProgressBar.start_link(total: 5, writer: writer)
      ProgressBar.send_event(tracker, :started, "test-pkg")
      ProgressBar.send_event(tracker, :finished, "test-pkg", "ok")
      Process.sleep(50)
      ProgressBar.stop(tracker)

      {_input, output} = StringIO.contents(writer)
      StringIO.close(writer)
      assert output =~ "test-pkg"
      assert output =~ "1/5"
    end

    test "nil safety through public API" do
      assert :ok == ProgressBar.send_event(nil, :started, "test")
      assert nil == ProgressBar.child(nil, 5, "test")
      assert :ok == ProgressBar.finish(nil)
      assert :ok == ProgressBar.stop(nil)
    end
  end

  # ---------------------------------------------------------------------------
  # Tests for edge cases
  # ---------------------------------------------------------------------------

  describe "edge cases" do
    test "tracker with total of 1 goes from 0% to 100%" do
      output = run_tracker(1, [{:started, "only"}, {:finished, "only", "ok"}])
      full_bar = String.duplicate("\u2588", 20)
      assert output =~ full_bar
      assert output =~ "1/1"
    end

    test "finishing an item that was never started still increments completed" do
      output = run_tracker(1, [{:finished, "ghost", "ok"}])
      assert output =~ "1/1"
    end

    test "activity shows done when completed equals total" do
      output = run_tracker(2, [{:skipped, "a"}, {:skipped, "b"}])
      assert output =~ "done"
    end

    test "activity shows waiting when nothing in flight and not complete" do
      output = run_tracker(5, [])
      assert output =~ "waiting..."
    end
  end
end
