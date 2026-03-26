# frozen_string_literal: true

# ============================================================================
# Network Stack — Full Layered Networking
# ============================================================================
#
# This is the entry point for the coding_adventures_network_stack gem.
# It loads all layers of the networking stack, from raw Ethernet frames
# (Layer 2) through HTTP (Layer 7).
#
# The layers are loaded bottom-up: each layer depends on the one below it,
# just like in a real network stack. The Ethernet layer knows nothing about
# TCP; the TCP layer knows nothing about HTTP. This separation of concerns
# is what makes networking work at Internet scale.
#
# ============================================================================

require_relative "coding_adventures/network_stack/version"
require_relative "coding_adventures/network_stack/ethernet"
require_relative "coding_adventures/network_stack/ipv4"
require_relative "coding_adventures/network_stack/tcp"
require_relative "coding_adventures/network_stack/udp"
require_relative "coding_adventures/network_stack/socket_api"
require_relative "coding_adventures/network_stack/dns"
require_relative "coding_adventures/network_stack/http"
require_relative "coding_adventures/network_stack/network_wire"
