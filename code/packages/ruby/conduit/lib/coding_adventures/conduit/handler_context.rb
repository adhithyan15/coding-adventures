# frozen_string_literal: true

require "json"

module CodingAdventures
  module Conduit
    # Evaluation context for all route handler and filter blocks.
    #
    # Every block is instance_exec'd inside a fresh HandlerContext, so helpers
    # defined here (json, html, halt, redirect, text) are available without an
    # explicit receiver. Existing blocks that accept a |request| argument still
    # work — instance_exec passes the request as the first argument.
    #
    #   # Both styles work:
    #   get "/greet/:name" do |request|   # explicit param
    #     json({ name: request.params["name"] })
    #   end
    #
    #   get "/greet/:name" do             # arity-0: self is HandlerContext
    #     json({ name: params["name"] })
    #   end
    class HandlerContext
      attr_reader :request

      def initialize(request)
        @request = request
      end

      # Shorthand for request.params (route-captured named parameters).
      def params
        request.params
      end

      # Send a JSON response. JSON.generate serializes data; content-type is set
      # to application/json. Raises HaltError to exit the handler immediately.
      def json(data, status = 200)
        raise HaltError.new(
          status,
          JSON.generate(data),
          { "content-type" => "application/json; charset=utf-8" }
        )
      end

      # Send an HTML response. Raises HaltError.
      def html(content, status = 200)
        raise HaltError.new(
          status,
          content.to_s,
          { "content-type" => "text/html; charset=utf-8" }
        )
      end

      # Send a plain text response. Raises HaltError.
      def text(content, status = 200)
        raise HaltError.new(
          status,
          content.to_s,
          { "content-type" => "text/plain; charset=utf-8" }
        )
      end

      # Short-circuit immediately with the given status, body, and headers.
      def halt(status, body = "", headers = {})
        raise HaltError.new(status, body, headers)
      end

      # Redirect to url. Default status is 302 Found.
      #
      # SECURITY: This method does not validate the URL. If url is derived from
      # user-supplied input (e.g. a `return_to` query parameter), validate that
      # it is a trusted relative path or known origin before calling redirect —
      # otherwise an attacker can craft a link that bounces users to a phishing
      # site (open redirect / CWE-601).
      def redirect(url, status = 302)
        raise HaltError.new(status, "", { "location" => url.to_s })
      end
    end
  end
end
