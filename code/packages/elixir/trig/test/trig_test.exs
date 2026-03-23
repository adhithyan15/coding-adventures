defmodule TrigTest do
  use ExUnit.Case

  # ---------------------------------------------------------------------------
  # Constants
  # ---------------------------------------------------------------------------

  @pi Trig.pi()
  @tolerance 1.0e-10

  # ---------------------------------------------------------------------------
  # pi/0
  # ---------------------------------------------------------------------------

  describe "pi/0" do
    test "returns pi to double precision" do
      assert Trig.pi() == 3.141592653589793
    end
  end

  # ---------------------------------------------------------------------------
  # sin/1 — basic values
  # ---------------------------------------------------------------------------

  describe "sin/1" do
    test "sin(0) is 0" do
      assert_in_delta Trig.sin(0), 0.0, @tolerance
    end

    test "sin(pi/6) is 0.5" do
      assert_in_delta Trig.sin(@pi / 6), 0.5, @tolerance
    end

    test "sin(pi/4) is sqrt(2)/2" do
      assert_in_delta Trig.sin(@pi / 4), :math.sqrt(2) / 2, @tolerance
    end

    test "sin(pi/2) is 1" do
      assert_in_delta Trig.sin(@pi / 2), 1.0, @tolerance
    end

    test "sin(pi) is 0" do
      assert_in_delta Trig.sin(@pi), 0.0, @tolerance
    end

    test "sin(3*pi/2) is -1" do
      assert_in_delta Trig.sin(3 * @pi / 2), -1.0, @tolerance
    end

    test "sin(2*pi) is 0" do
      assert_in_delta Trig.sin(2 * @pi), 0.0, @tolerance
    end

    # -- Symmetry: sin is an odd function: sin(-x) == -sin(x)
    test "sin(-x) == -sin(x) (odd function)" do
      for x <- [0.5, 1.0, 1.5, 2.0, 2.7] do
        assert_in_delta Trig.sin(-x), -Trig.sin(x), @tolerance
      end
    end

    # -- Large inputs: range reduction must work
    test "sin of large positive input" do
      assert_in_delta Trig.sin(1000.0), :math.sin(1000.0), 1.0e-8
    end

    test "sin of large negative input" do
      assert_in_delta Trig.sin(-1000.0), :math.sin(-1000.0), 1.0e-8
    end

    # -- Integer input
    test "sin accepts integers" do
      assert_in_delta Trig.sin(1), :math.sin(1), @tolerance
    end
  end

  # ---------------------------------------------------------------------------
  # cos/1 — basic values
  # ---------------------------------------------------------------------------

  describe "cos/1" do
    test "cos(0) is 1" do
      assert_in_delta Trig.cos(0), 1.0, @tolerance
    end

    test "cos(pi/3) is 0.5" do
      assert_in_delta Trig.cos(@pi / 3), 0.5, @tolerance
    end

    test "cos(pi/4) is sqrt(2)/2" do
      assert_in_delta Trig.cos(@pi / 4), :math.sqrt(2) / 2, @tolerance
    end

    test "cos(pi/2) is 0" do
      assert_in_delta Trig.cos(@pi / 2), 0.0, @tolerance
    end

    test "cos(pi) is -1" do
      assert_in_delta Trig.cos(@pi), -1.0, @tolerance
    end

    test "cos(2*pi) is 1" do
      assert_in_delta Trig.cos(2 * @pi), 1.0, @tolerance
    end

    # -- Symmetry: cos is an even function: cos(-x) == cos(x)
    test "cos(-x) == cos(x) (even function)" do
      for x <- [0.5, 1.0, 1.5, 2.0, 2.7] do
        assert_in_delta Trig.cos(-x), Trig.cos(x), @tolerance
      end
    end

    # -- Large inputs
    test "cos of large positive input" do
      assert_in_delta Trig.cos(1000.0), :math.cos(1000.0), 1.0e-8
    end

    test "cos of large negative input" do
      assert_in_delta Trig.cos(-1000.0), :math.cos(-1000.0), 1.0e-8
    end

    # -- Integer input
    test "cos accepts integers" do
      assert_in_delta Trig.cos(1), :math.cos(1), @tolerance
    end
  end

  # ---------------------------------------------------------------------------
  # Pythagorean identity: sin^2(x) + cos^2(x) == 1
  # ---------------------------------------------------------------------------

  describe "Pythagorean identity" do
    test "sin^2(x) + cos^2(x) == 1 for various x" do
      for x <- [0.0, 0.5, 1.0, @pi / 4, @pi / 2, @pi, 2.5, -1.3, 5.0, 100.0] do
        s = Trig.sin(x)
        c = Trig.cos(x)
        assert_in_delta s * s + c * c, 1.0, 1.0e-8,
          "Failed for x=#{x}: sin^2 + cos^2 = #{s * s + c * c}"
      end
    end
  end

  # ---------------------------------------------------------------------------
  # radians/1 and degrees/1
  # ---------------------------------------------------------------------------

  describe "radians/1" do
    test "0 degrees is 0 radians" do
      assert_in_delta Trig.radians(0), 0.0, @tolerance
    end

    test "90 degrees is pi/2 radians" do
      assert_in_delta Trig.radians(90), @pi / 2, @tolerance
    end

    test "180 degrees is pi radians" do
      assert_in_delta Trig.radians(180), @pi, @tolerance
    end

    test "360 degrees is 2*pi radians" do
      assert_in_delta Trig.radians(360), 2 * @pi, @tolerance
    end

    test "negative degrees" do
      assert_in_delta Trig.radians(-45), -@pi / 4, @tolerance
    end
  end

  describe "degrees/1" do
    test "0 radians is 0 degrees" do
      assert_in_delta Trig.degrees(0), 0.0, @tolerance
    end

    test "pi/2 radians is 90 degrees" do
      assert_in_delta Trig.degrees(@pi / 2), 90.0, @tolerance
    end

    test "pi radians is 180 degrees" do
      assert_in_delta Trig.degrees(@pi), 180.0, @tolerance
    end

    test "2*pi radians is 360 degrees" do
      assert_in_delta Trig.degrees(2 * @pi), 360.0, @tolerance
    end

    test "negative radians" do
      assert_in_delta Trig.degrees(-@pi / 4), -45.0, @tolerance
    end
  end

  # ---------------------------------------------------------------------------
  # Round-trip: radians(degrees(x)) == x
  # ---------------------------------------------------------------------------

  describe "round-trip conversion" do
    test "radians(degrees(x)) == x" do
      for x <- [0.0, 1.0, @pi / 3, @pi, 2 * @pi, -0.5] do
        assert_in_delta Trig.radians(Trig.degrees(x)), x, @tolerance
      end
    end

    test "degrees(radians(x)) == x" do
      for x <- [0.0, 45.0, 90.0, 180.0, 360.0, -30.0] do
        assert_in_delta Trig.degrees(Trig.radians(x)), x, @tolerance
      end
    end
  end
end
