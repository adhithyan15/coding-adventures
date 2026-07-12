// Tests for the C++ irc-proto parser/serializer, using the header-only
// iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <optional>
#include <string>
#include <vector>

#include "irc_proto.hpp"

namespace irc = ca::irc;

int main() {
    // ── simple command with one param ────────────────────────────────────
    {
        irc::Message m = irc::parse("NICK alice");
        ISO_CHECK(!m.prefix.has_value());
        ISO_CHECK_STR_EQ(m.command.c_str(), "NICK");
        ISO_CHECK_EQ_UINT(m.params.size(), 1u);
        ISO_CHECK_STR_EQ(m.params[0].c_str(), "alice");
    }

    // ── command is upper-cased ───────────────────────────────────────────
    {
        irc::Message m = irc::parse("join #general");
        ISO_CHECK_STR_EQ(m.command.c_str(), "JOIN");
        ISO_CHECK_STR_EQ(m.params[0].c_str(), "#general");
    }

    // ── server prefix + numeric + trailing ───────────────────────────────
    {
        irc::Message m = irc::parse(":irc.local 001 alice :Welcome to the network!");
        ISO_CHECK(m.prefix.has_value());
        ISO_CHECK_STR_EQ(m.prefix->c_str(), "irc.local");
        ISO_CHECK_STR_EQ(m.command.c_str(), "001");
        ISO_CHECK_EQ_UINT(m.params.size(), 2u);
        ISO_CHECK_STR_EQ(m.params[0].c_str(), "alice");
        ISO_CHECK_STR_EQ(m.params[1].c_str(), "Welcome to the network!");
    }

    // ── user-mask prefix ─────────────────────────────────────────────────
    {
        irc::Message m = irc::parse(":alice!alice@127.0.0.1 PRIVMSG #chan :hello world");
        ISO_CHECK_STR_EQ(m.prefix->c_str(), "alice!alice@127.0.0.1");
        ISO_CHECK_STR_EQ(m.command.c_str(), "PRIVMSG");
        ISO_CHECK_STR_EQ(m.params[0].c_str(), "#chan");
        ISO_CHECK_STR_EQ(m.params[1].c_str(), "hello world");
    }

    // ── trailing param preserves interior spaces ─────────────────────────
    {
        irc::Message m = irc::parse("PRIVMSG #chan :hello   world   !");
        ISO_CHECK_EQ_UINT(m.params.size(), 2u);
        ISO_CHECK_STR_EQ(m.params[1].c_str(), "hello   world   !");
    }

    // ── no params ────────────────────────────────────────────────────────
    {
        irc::Message m = irc::parse("PING");
        ISO_CHECK_STR_EQ(m.command.c_str(), "PING");
        ISO_CHECK(m.params.empty());
    }

    // ── USER command ─────────────────────────────────────────────────────
    {
        irc::Message m = irc::parse("USER alice 0 * :Alice Smith");
        ISO_CHECK_STR_EQ(m.command.c_str(), "USER");
        std::vector<std::string> want = {"alice", "0", "*", "Alice Smith"};
        ISO_CHECK(m.params == want);
    }

    // ── the 15-parameter cap ─────────────────────────────────────────────
    {
        std::string line = "CMD";
        for (int i = 0; i < 16; i++) line += " x";
        irc::Message m = irc::parse(line);
        ISO_CHECK_EQ_UINT(m.params.size(), irc::MAX_PARAMS);
    }

    // ── empty trailing param ─────────────────────────────────────────────
    {
        irc::Message m = irc::parse("AWAY :");
        ISO_CHECK_EQ_UINT(m.params.size(), 1u);
        ISO_CHECK_STR_EQ(m.params[0].c_str(), "");
    }

    // ── numeric command with mask + trailing ─────────────────────────────
    {
        irc::Message m = irc::parse(":server.local 433 * nick :Nickname is already in use");
        ISO_CHECK_STR_EQ(m.command.c_str(), "433");
        std::vector<std::string> want = {"*", "nick", "Nickname is already in use"};
        ISO_CHECK(m.params == want);
    }

    // ── error cases (throwing + try_parse) ───────────────────────────────
    {
        const char* bad[] = {"", "   ", ":irc.local"};
        for (const char* b : bad) {
            ISO_CHECK(!irc::try_parse(b).has_value());
            bool threw = false;
            try {
                (void)irc::parse(b);
            } catch (const irc::ParseError&) {
                threw = true;
            }
            ISO_CHECK(threw);
        }
    }

    // ── serialization ────────────────────────────────────────────────────
    {
        irc::Message m;
        m.command = "NICK";
        m.params = {"alice"};
        ISO_CHECK_STR_EQ(irc::serialize(m).c_str(), "NICK alice\r\n");
    }
    {
        irc::Message m;
        m.prefix = "alice!alice@host";
        m.command = "PRIVMSG";
        m.params = {"#chan", "hello world"};
        ISO_CHECK_STR_EQ(irc::serialize(m).c_str(),
                         ":alice!alice@host PRIVMSG #chan :hello world\r\n");
    }
    {
        irc::Message m;
        m.command = "PING";
        ISO_CHECK_STR_EQ(irc::serialize(m).c_str(), "PING\r\n");
    }
    {
        irc::Message m;
        m.command = "AWAY";
        m.params = {""};
        ISO_CHECK_STR_EQ(irc::serialize(m).c_str(), "AWAY :\r\n");
    }

    // ── round-trips ──────────────────────────────────────────────────────
    {
        std::string wire = ":alice!alice@host PRIVMSG #chan :hello world\r\n";
        irc::Message m = irc::parse(wire.substr(0, wire.size() - 2));
        ISO_CHECK_STR_EQ(irc::serialize(m).c_str(), wire.c_str());
    }
    {
        irc::Message m = irc::parse("NICK alice");
        ISO_CHECK_STR_EQ(irc::serialize(m).c_str(), "NICK alice\r\n");
    }

    return ISO_TEST_RESULT();
}
