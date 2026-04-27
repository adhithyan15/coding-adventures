# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00::ParseCache
# ================================================================
#
# # Why Cache Parse Results?
#
# Parsing is the most expensive operation in a language server. For a large
# file, parsing on every keystroke would lag the editor noticeably.
#
# The LSP protocol helps by sending a version number with every change. If
# the document hasn't changed (same URI, same version), the parse result
# from the previous keystroke is still valid.
#
# # Cache Key Design
#
# The cache key is [uri, version]. Version is a monotonically increasing
# integer that the editor increments with each change. Using version in
# the key means:
#
#   Same [uri, version] -> cache hit -> return cached result
#   Different version   -> cache miss -> re-parse and cache new result
#
# The old entry is evicted when a new version is cached for the same URI.
# This keeps memory bounded at O(open_documents) entries.
#
# # Thread Safety
#
# The ParseCache is NOT thread-safe. This is intentional: the LspServer
# processes one message at a time (single-threaded), so no locking is needed.
#
# ================================================================

module CodingAdventures
  module Ls00
    # ParseResult holds the outcome of parsing one version of a document.
    #
    # Even on parse error, we store the partial AST and diagnostics so that
    # other features (hover, folding, symbols) can still work on valid portions.
    ParseResult = Struct.new(:ast, :diagnostics, :error, keyword_init: true)

    class ParseCache
      def initialize
        @cache = {} # [uri, version] -> ParseResult
      end

      # get_or_parse returns the parse result for (uri, version).
      #
      # If the result is already cached, it is returned immediately without
      # calling the bridge again. Otherwise, bridge.parse(source) is called,
      # the result is stored, and previous cache entries for this URI are
      # evicted to prevent unbounded growth.
      def get_or_parse(uri, version, source, bridge)
        key = [uri, version]

        # Cache hit: the document hasn't changed since last parse.
        return @cache[key] if @cache.key?(key)

        # Cache miss: parse and store. Evict any stale entry for this URI first.
        evict(uri)

        ast, diags = bridge.parse(source)
        diags ||= [] # normalize nil to empty array for JSON

        result = ParseResult.new(ast: ast, diagnostics: diags, error: nil)
        @cache[key] = result
        result
      end

      # evict removes all cached entries for a given URI.
      #
      # Called when a document is closed (didClose) so the cache entry is
      # cleaned up. Also called internally before adding a new entry.
      def evict(uri)
        @cache.delete_if { |k, _| k[0] == uri }
      end
    end
  end
end
