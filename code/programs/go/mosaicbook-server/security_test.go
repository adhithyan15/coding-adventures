// security_test.go — tests for requireLocalOrigin / isLocalHost (#13178)

package main

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestIsLocalHost(t *testing.T) {
	cases := []struct {
		in   string
		want bool
	}{
		{"localhost", true},
		{"localhost:7331", true},
		{"127.0.0.1", true},
		{"127.0.0.1:7331", true},
		{"[::1]:7331", true},
		{"::1", true},
		{"", false},
		{"evil.example.com", false},
		{"evil.example.com:7331", false},
		// A DNS-rebinding-style attempt to look local via a subdomain —
		// SplitHostPort/hostname comparison must be exact, not a prefix or
		// suffix match.
		{"127.0.0.1.evil.com", false},
		{"localhost.evil.com", false},
		{"notlocalhost", false},
	}
	for _, tc := range cases {
		if got := isLocalHost(tc.in); got != tc.want {
			t.Errorf("isLocalHost(%q) = %v, want %v", tc.in, got, tc.want)
		}
	}
}

func newGuardedTestHandler(t *testing.T) (http.Handler, *bool) {
	t.Helper()
	called := false
	inner := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		called = true
		w.WriteHeader(http.StatusOK)
	})
	return requireLocalOrigin(inner), &called
}

func TestRequireLocalOrigin_RejectsForeignHost(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	// httptest.NewRequest defaults Host to "example.com" when the target is
	// a bare path — exactly the shape of a request that arrived with a
	// forged/rebound Host header naming some other domain.
	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusForbidden)
	}
	if *called {
		t.Error("inner handler should not have been called for a foreign Host")
	}
}

func TestRequireLocalOrigin_AllowsLocalhostNoOrigin(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	req.Host = "localhost:7331"
	// No Origin header — the common case for same-origin page loads and
	// plain curl requests.
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusOK)
	}
	if !*called {
		t.Error("inner handler should have been called for a local Host with no Origin")
	}
}

func TestRequireLocalOrigin_RejectsForeignOriginEvenWithLocalHost(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	// This is the shape that matters most: a page on some other site issues
	// fetch("http://localhost:7331/api/degradations/...") directly. The Host
	// header names this server correctly, but Origin reveals the request
	// didn't originate from this server's own page.
	req := httptest.NewRequest(http.MethodGet, "/api/degradations/xaml/Button", nil)
	req.Host = "localhost:7331"
	req.Header.Set("Origin", "http://evil.example.com")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusForbidden)
	}
	if *called {
		t.Error("inner handler should not have been called for a foreign Origin")
	}
}

func TestRequireLocalOrigin_AllowsLocalHostAndLocalOrigin(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	req.Host = "localhost:7331"
	req.Header.Set("Origin", "http://localhost:7331")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusOK)
	}
	if !*called {
		t.Error("inner handler should have been called for a local Host and local Origin")
	}
}

// This is the case a first version of this middleware missed: a hostile
// page embeds <iframe src="http://localhost:PORT/preview/...">. Iframe
// navigations carry no Origin header at all (per the Fetch spec), so a
// Host+Origin-only check lets this straight through to a handler that
// spawns a real mosaic-compile subprocess per request — exactly the DoS
// #13178 exists to prevent, and exactly how this tool's own browser shell
// legitimately loads /preview/ (static/index.html sets iframe.src to it).
// Modern browsers do send Sec-Fetch-Site on navigations, unlike Origin;
// this is what actually needs to catch it.
func TestRequireLocalOrigin_RejectsCrossSiteIframeNavigation(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	req := httptest.NewRequest(http.MethodGet, "/preview/html/Button/Default", nil)
	req.Host = "localhost:7331"
	// No Origin header — this is the point of the test.
	req.Header.Set("Sec-Fetch-Site", "cross-site")
	req.Header.Set("Sec-Fetch-Mode", "navigate")
	req.Header.Set("Sec-Fetch-Dest", "iframe")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusForbidden)
	}
	if *called {
		t.Error("inner handler should not have been called for a cross-site Sec-Fetch-Site navigation")
	}
}

func TestRequireLocalOrigin_AllowsSameOriginSecFetchSite(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	req.Host = "localhost:7331"
	req.Header.Set("Sec-Fetch-Site", "same-origin")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusOK)
	}
	if !*called {
		t.Error("inner handler should have been called for Sec-Fetch-Site: same-origin")
	}
}

// A direct top-level navigation (typing the URL, a bookmark) sends
// Sec-Fetch-Site: none, not "cross-site" — must not be rejected.
func TestRequireLocalOrigin_AllowsSecFetchSiteNone(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	req.Host = "localhost:7331"
	req.Header.Set("Sec-Fetch-Site", "none")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusOK {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusOK)
	}
	if !*called {
		t.Error("inner handler should have been called for Sec-Fetch-Site: none")
	}
}

func TestRequireLocalOrigin_RejectsMalformedOrigin(t *testing.T) {
	handler, called := newGuardedTestHandler(t)

	req := httptest.NewRequest(http.MethodGet, "/api/stories", nil)
	req.Host = "localhost:7331"
	req.Header.Set("Origin", "not a valid url \x7f")
	rec := httptest.NewRecorder()
	handler.ServeHTTP(rec, req)

	if rec.Code != http.StatusForbidden {
		t.Errorf("status: got %d, want %d", rec.Code, http.StatusForbidden)
	}
	if *called {
		t.Error("inner handler should not have been called for a malformed Origin")
	}
}
