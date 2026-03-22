# frozen_string_literal: true

# ============================================================================
# DNS — Domain Name System (Simplified Static Resolver)
# ============================================================================
#
# DNS is the Internet's phone book. When you type "example.com" into your
# browser, your computer needs to find out the IP address of example.com's
# server. DNS handles this name-to-address translation.
#
# In a real system, DNS resolution involves:
#   1. Checking the local cache
#   2. Asking a recursive DNS resolver (usually your ISP's)
#   3. The resolver queries root servers, TLD servers, and authoritative servers
#   4. The answer propagates back through the chain
#
# For our simulation, we use a static lookup table — a simple hash map from
# hostname to IP address. This avoids the complexity of the full DNS protocol
# while preserving the essential concept: names map to addresses.
#
# Default entries:
#   "localhost" -> [127, 0, 0, 1]
#
# ============================================================================

module CodingAdventures
  module NetworkStack
    class DNSResolver
      attr_reader :static_table

      def initialize
        @static_table = {
          "localhost" => [127, 0, 0, 1]
        }
      end

      # Resolve a hostname to an IP address.
      #
      # Returns a 4-byte IP address array, or nil if the hostname is unknown.
      #
      # In a real DNS resolver, a miss in the static table would trigger a
      # UDP query to a DNS server on port 53. Our simulation just returns nil
      # for unknown hostnames.
      #
      def resolve(hostname)
        @static_table[hostname]&.dup
      end

      # Add a static hostname-to-IP mapping.
      #
      # This is like editing /etc/hosts on a Unix system — it takes
      # precedence over any DNS server query.
      #
      def add_static(hostname, ip)
        @static_table[hostname] = ip.dup
      end
    end
  end
end
