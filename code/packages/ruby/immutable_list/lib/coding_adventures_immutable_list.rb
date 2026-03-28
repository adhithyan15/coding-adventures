# frozen_string_literal: true

require_relative "coding_adventures/immutable_list/version"
require_relative "coding_adventures/immutable_list/persistent_vector"

module CodingAdventures
  # Persistent vector using a 32-way trie with structural sharing,
  # inspired by Clojure's PersistentVector.
  #
  # Usage:
  #
  #   list = CodingAdventures::ImmutableList.empty
  #   list2 = list.push("hello")
  #   list3 = list2.push("world")
  #   list3.get(0)   # => "hello"
  #   list3.get(1)   # => "world"
  #   list2.size     # => 1  (list2 is unchanged)
  #
  # See CodingAdventures::ImmutableList::PersistentVector for the full API.
  module ImmutableList
    # Build an empty list.
    # @return [PersistentVector]
    def self.empty
      PersistentVector.empty
    end

    # Build a list from a Ruby Array.
    # @param arr [Array]
    # @return [PersistentVector]
    def self.from_array(arr)
      PersistentVector.from_array(arr)
    end

    # Variadic constructor — ImmutableList.of("a", "b", "c")
    # @return [PersistentVector]
    def self.of(*elements)
      PersistentVector.from_array(elements)
    end
  end
end
