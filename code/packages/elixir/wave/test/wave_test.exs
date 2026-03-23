defmodule WaveTest do
  use ExUnit.Case, async: true

  # ---------------------------------------------------------------------------
  # Helper: pi from our Trig library
  # ---------------------------------------------------------------------------

  defp pi, do: Trig.pi()

  # ---------------------------------------------------------------------------
  # Constructor Tests
  # ---------------------------------------------------------------------------

  describe "Wave.new/3" do
    test "creates a wave with default phase of 0" do
      assert {:ok, %Wave{amplitude: 1.0, frequency: 440.0, phase: +0.0}} =
               Wave.new(1.0, 440.0)
    end

    test "creates a wave with explicit phase" do
      assert {:ok, %Wave{amplitude: 5.0, frequency: 2.0, phase: 1.5}} =
               Wave.new(5.0, 2.0, 1.5)
    end

    test "allows zero amplitude (a flat line)" do
      assert {:ok, %Wave{amplitude: +0.0, frequency: 1.0, phase: +0.0}} =
               Wave.new(0.0, 1.0)
    end

    test "rejects negative amplitude" do
      assert {:error, "amplitude must be non-negative"} = Wave.new(-1.0, 440.0)
    end

    test "rejects zero frequency" do
      assert {:error, "frequency must be positive"} = Wave.new(1.0, 0.0)
    end

    test "rejects negative frequency" do
      assert {:error, "frequency must be positive"} = Wave.new(1.0, -5.0)
    end
  end

  # ---------------------------------------------------------------------------
  # Derived Quantities
  # ---------------------------------------------------------------------------

  describe "Wave.period/1" do
    test "period of 1 Hz wave is 1 second" do
      {:ok, wave} = Wave.new(1.0, 1.0)
      assert_in_delta Wave.period(wave), 1.0, 1.0e-10
    end

    test "period of 4 Hz wave is 0.25 seconds" do
      {:ok, wave} = Wave.new(1.0, 4.0)
      assert_in_delta Wave.period(wave), 0.25, 1.0e-10
    end

    test "period of 1000 Hz wave is 0.001 seconds" do
      {:ok, wave} = Wave.new(1.0, 1000.0)
      assert_in_delta Wave.period(wave), 0.001, 1.0e-10
    end
  end

  describe "Wave.angular_frequency/1" do
    test "angular frequency of 1 Hz wave is 2*pi" do
      {:ok, wave} = Wave.new(1.0, 1.0)
      assert_in_delta Wave.angular_frequency(wave), 2.0 * pi(), 1.0e-10
    end

    test "angular frequency of 0.5 Hz wave is pi" do
      {:ok, wave} = Wave.new(1.0, 0.5)
      assert_in_delta Wave.angular_frequency(wave), pi(), 1.0e-10
    end

    test "angular frequency of 440 Hz wave" do
      {:ok, wave} = Wave.new(1.0, 440.0)
      assert_in_delta Wave.angular_frequency(wave), 2.0 * pi() * 440.0, 1.0e-10
    end
  end

  # ---------------------------------------------------------------------------
  # Evaluation Tests
  # ---------------------------------------------------------------------------

  describe "Wave.evaluate/2 — basic 1 Hz wave (A=1, f=1, phase=0)" do
    setup do
      {:ok, wave} = Wave.new(1.0, 1.0)
      %{wave: wave}
    end

    test "at t=0 gives 0 (wave starts at zero crossing)", %{wave: wave} do
      # sin(0) = 0
      assert_in_delta Wave.evaluate(wave, 0.0), 0.0, 1.0e-10
    end

    test "at t=0.25 gives 1.0 (quarter period = peak)", %{wave: wave} do
      # sin(2*pi*1*0.25) = sin(pi/2) = 1.0
      assert_in_delta Wave.evaluate(wave, 0.25), 1.0, 1.0e-10
    end

    test "at t=0.5 gives 0 (half period = zero crossing)", %{wave: wave} do
      # sin(2*pi*1*0.5) = sin(pi) = 0.0
      assert_in_delta Wave.evaluate(wave, 0.5), 0.0, 1.0e-10
    end

    test "at t=0.75 gives -1.0 (three-quarter period = trough)", %{wave: wave} do
      # sin(2*pi*1*0.75) = sin(3*pi/2) = -1.0
      assert_in_delta Wave.evaluate(wave, 0.75), -1.0, 1.0e-10
    end

    test "at t=1.0 gives 0 (full period = back to start)", %{wave: wave} do
      # sin(2*pi*1*1.0) = sin(2*pi) = 0.0
      assert_in_delta Wave.evaluate(wave, 1.0), 0.0, 1.0e-10
    end
  end

  describe "Wave.evaluate/2 — periodicity" do
    test "wave repeats after one period" do
      {:ok, wave} = Wave.new(3.0, 5.0)
      period = Wave.period(wave)

      # Check at several time points that y(t) == y(t + T)
      for t <- [0.0, 0.01, 0.05, 0.1, 0.15, 0.19] do
        assert_in_delta Wave.evaluate(wave, t), Wave.evaluate(wave, t + period), 1.0e-10,
          "Wave should repeat at t=#{t} and t=#{t + period}"
      end
    end

    test "wave repeats after multiple periods" do
      {:ok, wave} = Wave.new(2.0, 3.0)
      period = Wave.period(wave)

      assert_in_delta Wave.evaluate(wave, 0.1), Wave.evaluate(wave, 0.1 + 5 * period), 1.0e-9
    end
  end

  describe "Wave.evaluate/2 — phase shift" do
    test "phase pi/2 starts at peak (cosine-like)" do
      # sin(0 + pi/2) = sin(pi/2) = 1.0
      {:ok, wave} = Wave.new(1.0, 1.0, pi() / 2.0)
      assert_in_delta Wave.evaluate(wave, 0.0), 1.0, 1.0e-10
    end

    test "phase pi starts at zero but inverted" do
      # sin(pi/2 + pi) = sin(3*pi/2) = -1.0
      {:ok, wave} = Wave.new(1.0, 1.0, pi())
      assert_in_delta Wave.evaluate(wave, 0.25), -1.0, 1.0e-10
    end

    test "phase -pi/2 starts at trough" do
      # sin(0 - pi/2) = sin(-pi/2) = -1.0
      {:ok, wave} = Wave.new(1.0, 1.0, -pi() / 2.0)
      assert_in_delta Wave.evaluate(wave, 0.0), -1.0, 1.0e-10
    end
  end

  describe "Wave.evaluate/2 — amplitude scaling" do
    test "amplitude 5 scales the wave to [-5, 5]" do
      {:ok, wave} = Wave.new(5.0, 1.0)

      # At the peak (t=0.25), the value should be 5.0
      assert_in_delta Wave.evaluate(wave, 0.25), 5.0, 1.0e-10

      # At the trough (t=0.75), the value should be -5.0
      assert_in_delta Wave.evaluate(wave, 0.75), -5.0, 1.0e-10
    end

    test "amplitude 0 gives a flat line at zero" do
      {:ok, wave} = Wave.new(0.0, 1.0)

      assert_in_delta Wave.evaluate(wave, 0.0), 0.0, 1.0e-10
      assert_in_delta Wave.evaluate(wave, 0.25), 0.0, 1.0e-10
      assert_in_delta Wave.evaluate(wave, 0.5), 0.0, 1.0e-10
    end
  end

  describe "Wave.evaluate/2 — different frequencies" do
    test "2 Hz wave completes a cycle in 0.5 seconds" do
      {:ok, wave} = Wave.new(1.0, 2.0)

      # At t=0.125 (quarter of the 0.5s period), should be at peak
      assert_in_delta Wave.evaluate(wave, 0.125), 1.0, 1.0e-10

      # At t=0.5 (full period), back to zero
      assert_in_delta Wave.evaluate(wave, 0.5), 0.0, 1.0e-10
    end

    test "0.5 Hz wave has a 2-second period" do
      {:ok, wave} = Wave.new(1.0, 0.5)

      # Peak at t=0.5 (quarter of 2-second period)
      assert_in_delta Wave.evaluate(wave, 0.5), 1.0, 1.0e-10
    end
  end

  describe "Wave.evaluate/2 — negative time" do
    test "negative time works (wave extends into the past)" do
      {:ok, wave} = Wave.new(1.0, 1.0)

      # sin(-pi/2) = -1.0 at t=-0.25
      assert_in_delta Wave.evaluate(wave, -0.25), -1.0, 1.0e-10
    end
  end
end
