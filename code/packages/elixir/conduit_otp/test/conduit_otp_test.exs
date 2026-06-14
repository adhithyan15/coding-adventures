defmodule CodingAdventures.ConduitOtpTest do
  @moduledoc """
  Smoke tests for the top-level umbrella module.
  """

  use ExUnit.Case, async: true

  test "application/0 returns the Application module" do
    assert CodingAdventures.ConduitOtp.application() ==
             CodingAdventures.ConduitOtp.Application
  end
end
