// Tests for the C++ url-parser, using the iso_test.h harness. Cases are taken
// from the Rust crate's own tests.
#include "iso_test.h"

#include <optional>
#include <string>

#include "url_parser.hpp"

namespace url = ca::url;
using Opt = std::optional<std::string>;

int main() {
    // Simple http URL.
    {
        url::Url u = url::Url::parse("http://www.example.com");
        ISO_CHECK(u.scheme == "http");
        ISO_CHECK(u.host == Opt("www.example.com"));
        ISO_CHECK(!u.port.has_value());
        ISO_CHECK(u.path == "/");
        ISO_CHECK(!u.query.has_value());
        ISO_CHECK(!u.fragment.has_value());
    }

    // All components.
    {
        url::Url u = url::Url::parse(
            "http://alice:secret@www.example.com:8080/docs/page.html"
            "?q=hello#section2");
        ISO_CHECK(u.scheme == "http");
        ISO_CHECK(u.userinfo == Opt("alice:secret"));
        ISO_CHECK(u.host == Opt("www.example.com"));
        ISO_CHECK(u.port == std::optional<std::uint16_t>(8080));
        ISO_CHECK(u.path == "/docs/page.html");
        ISO_CHECK(u.query == Opt("q=hello"));
        ISO_CHECK(u.fragment == Opt("section2"));
    }

    // Lowercasing.
    {
        url::Url u = url::Url::parse("HTTP://WWW.EXAMPLE.COM/PATH");
        ISO_CHECK(u.scheme == "http");
        ISO_CHECK(u.host == Opt("www.example.com"));
        ISO_CHECK(u.path == "/PATH");
    }

    // mailto.
    {
        url::Url u = url::Url::parse("mailto:alice@example.com");
        ISO_CHECK(u.scheme == "mailto");
        ISO_CHECK(!u.host.has_value());
        ISO_CHECK(u.path == "alice@example.com");
    }

    // effective_port.
    {
        ISO_CHECK(url::Url::parse("http://example.com").effective_port() ==
                  std::optional<std::uint16_t>(80));
        ISO_CHECK(url::Url::parse("https://x.com/login").effective_port() ==
                  std::optional<std::uint16_t>(443));
        ISO_CHECK(url::Url::parse("ftp://x.com/y").effective_port() ==
                  std::optional<std::uint16_t>(21));
        url::Url u = url::Url::parse("http://example.com:9090");
        ISO_CHECK(u.port == std::optional<std::uint16_t>(9090));
        ISO_CHECK(u.effective_port() == std::optional<std::uint16_t>(9090));
    }

    // authority().
    {
        ISO_CHECK(url::Url::parse("http://user:pass@host.com:8080/path")
                      .authority() == "user:pass@host.com:8080");
        ISO_CHECK(url::Url::parse("http://host.com/path").authority() ==
                  "host.com");
    }

    // Errors.
    {
        auto err = [](const std::string& in, url::Error k) {
            bool got = false;
            try {
                url::Url::parse(in);
            } catch (const url::ParseError& e) {
                got = e.kind == k;
            }
            return got;
        };
        ISO_CHECK(err("www.example.com", url::Error::MissingScheme));
        ISO_CHECK(err("1http://x.com", url::Error::InvalidScheme));
        ISO_CHECK(err("http://host:99999", url::Error::InvalidPort));
    }

    // Percent-encoding.
    {
        ISO_CHECK(url::percent_encode("hello world") == "hello%20world");
        ISO_CHECK(url::percent_encode("abc-def_ghi.jkl~mno") ==
                  "abc-def_ghi.jkl~mno");
        ISO_CHECK(url::percent_encode("/path/to/file") == "/path/to/file");
    }

    // Percent-decoding.
    {
        ISO_CHECK(url::percent_decode("hello%20world") == "hello world");
        ISO_CHECK(url::percent_decode("%E6%97%A5") == "\xe6\x97\xa5"); // 日
        bool threw = false;
        try {
            url::percent_decode("%2");
        } catch (const url::ParseError&) {
            threw = true;
        }
        ISO_CHECK(threw);
        threw = false;
        try {
            url::percent_decode("%GG");
        } catch (const url::ParseError&) {
            threw = true;
        }
        ISO_CHECK(threw);
        // round trip
        std::string original = "path with spaces & special=chars!";
        ISO_CHECK(url::percent_decode(url::percent_encode(original)) == original);
    }

    // Resolve.
    {
        url::Url base = url::Url::parse("http://host/a/b/c.html");
        ISO_CHECK(base.resolve("d.html").path == "/a/b/d.html");
        ISO_CHECK(base.resolve("../d.html").path == "/a/d.html");
        ISO_CHECK(base.resolve("../../d.html").path == "/d.html");
        ISO_CHECK(base.resolve("/x/y.html").path == "/x/y.html");
        ISO_CHECK(base.resolve("./d").path == "/a/b/d");
        ISO_CHECK(base.resolve("d.html").host == Opt("host"));
    }
    {
        url::Url base = url::Url::parse("http://host/a/b");
        url::Url sr = base.resolve("//other.com/path");
        ISO_CHECK(sr.scheme == "http");
        ISO_CHECK(sr.host == Opt("other.com"));
        ISO_CHECK(sr.path == "/path");

        url::Url abs = base.resolve("https://other.com/x");
        ISO_CHECK(abs.scheme == "https");
        ISO_CHECK(abs.host == Opt("other.com"));
        ISO_CHECK(abs.path == "/x");

        url::Url frag = base.resolve("#sec");
        ISO_CHECK(frag.path == "/a/b");
        ISO_CHECK(frag.fragment == Opt("sec"));
    }
    {
        url::Url base = url::Url::parse("http://host/a/b?q=1#frag");
        url::Url e = base.resolve("");
        ISO_CHECK(e.path == "/a/b");
        ISO_CHECK(e.query == Opt("q=1"));
        ISO_CHECK(!e.fragment.has_value()); // stripped
    }

    // to_url_string round trip.
    {
        url::Url u = url::Url::parse("http://a:b@host.com:8080/p/q?x=1#f");
        ISO_CHECK(u.to_url_string() == "http://a:b@host.com:8080/p/q?x=1#f");
    }

    return ISO_TEST_RESULT();
}
