# frozen_string_literal: true

require "coding_adventures_chief_of_staff_channel_crypto"
require "coding_adventures_chief_of_staff_channel_store"
require "coding_adventures_sha256"

module CodingAdventures
  module ChiefOfStaffChannelEpochActivation
    VERSION = "0.1.0"

    # Short local names for the two profiles D18T composes. D18T deliberately
    # creates no parallel wire formats of its own -- everything below reuses
    # D18P records and D18Q grants.
    Store = CodingAdventures::ChiefOfStaffChannelStore
    Crypto = CodingAdventures::ChiefOfStaffChannelCrypto
  end
end

require_relative "coding_adventures_chief_of_staff_channel_epoch_activation/wire"
require_relative "coding_adventures_chief_of_staff_channel_epoch_activation/custody"
require_relative "coding_adventures_chief_of_staff_channel_epoch_activation/activation"
