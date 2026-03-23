#!/usr/bin/env ruby
# frozen_string_literal: true

# groups_tool.rb -- Print the groups a user is in
# ==================================================
#
# === What This Program Does ===
#
# This is a reimplementation of the `groups` utility. It prints the
# group memberships for each specified user, or for the current user
# if no user is given.
#
# === Output Format ===
#
# For a single user (or no argument):
#
#     $ groups
#     staff admin wheel
#
# For multiple users, each line is prefixed with the username:
#
#     $ groups alice bob
#     alice : staff admin
#     bob : staff developers
#
# === How It Works ===
#
# The `groups` command is essentially equivalent to `id -Gn`. We use
# Ruby's `Etc` module to look up group memberships:
#
# 1. Find the user's primary group from their passwd entry
# 2. Scan all groups in /etc/group for supplementary memberships
# 3. Print group names separated by spaces
#
# For the current user (no argument), we can use `Process.groups` as
# a shortcut, which returns the supplementary group list directly.

require "etc"
require "coding_adventures_cli_builder"

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

GROUPS_SPEC_FILE = File.join(File.dirname(__FILE__), "groups.json")

# ---------------------------------------------------------------------------
# Business Logic: get_user_groups
# ---------------------------------------------------------------------------
# Get the list of group names for a user.
#
# Parameters:
#   username - The username to look up, or nil for current user
#
# Returns: An array of group name strings, or nil if user not found.

def get_user_groups(username)
  if username
    # Look up the specified user
    begin
      pw = Etc.getpwnam(username)
    rescue ArgumentError
      warn "groups: '#{username}': no such user"
      return nil
    end

    primary_gid = pw.gid
    group_names = []

    # Add primary group
    begin
      group_names << Etc.getgrgid(primary_gid).name
    rescue ArgumentError
      group_names << primary_gid.to_s
    end

    # Scan all groups for supplementary membership
    Etc.group do |gr|
      if gr.mem.include?(username) && gr.gid != primary_gid
        group_names << gr.name
      end
    end

    group_names
  else
    # Current user: use Process.groups for the group ID list
    Process.groups.uniq.map do |gid|
      begin
        Etc.getgrgid(gid).name
      rescue ArgumentError
        gid.to_s
      end
    end
  end
end

# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------

def groups_main
  # --- Step 1: Parse arguments ---------------------------------------------
  begin
    result = CodingAdventures::CliBuilder::Parser.new(GROUPS_SPEC_FILE, ["groups"] + ARGV).parse
  rescue CodingAdventures::CliBuilder::ParseErrors => e
    e.errors.each { |err| warn "groups: #{err.message}" }
    exit 1
  end

  # --- Step 2: Dispatch on result type -------------------------------------
  case result
  when CodingAdventures::CliBuilder::HelpResult
    puts result.text
    exit 0
  when CodingAdventures::CliBuilder::VersionResult
    puts result.version
    exit 0
  when CodingAdventures::CliBuilder::ParseResult
    # --- Step 3: Business logic --------------------------------------------
    users = result.arguments.fetch("users", [])

    if users.empty?
      # No arguments: print current user's groups
      group_names = get_user_groups(nil)
      puts group_names.join(" ") if group_names
    else
      # One or more usernames specified
      users.each do |username|
        group_names = get_user_groups(username)
        next unless group_names

        if users.length == 1
          puts group_names.join(" ")
        else
          puts "#{username} : #{group_names.join(" ")}"
        end
      end
    end
  end
end

# Only run main when this file is executed directly.
groups_main if __FILE__ == $PROGRAM_NAME
