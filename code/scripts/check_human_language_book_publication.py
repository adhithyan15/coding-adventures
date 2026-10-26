#!/usr/bin/env python3
"""Read-only preflight for replay-safe Human Languages book publication.

The full-book build produces the bytes. This script runs in a separate job with
only ``contents: read`` and decides whether the write-token job may publish
them. In particular, a failed old run must not roll the shared Pages branch
back, and an immutable release must never be sent to an overwrite action.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode
from urllib.request import HTTPRedirectHandler, Request, build_opener

SHA = re.compile(r"[0-9a-f]{40}\Z")
REPOSITORY = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\Z")
PAGES_MESSAGE = re.compile(r"deploy: ([0-9a-f]{40})\Z")
RELEASE_TAG = re.compile(r"human-languages-v\d{4}\.\d{2}\.\d{2}\.(\d+)\Z")


class PublicationError(RuntimeError):
    """A publication state is ambiguous or would violate HL00."""


def parse_pages_source(message: str) -> str:
    """The path's last deploy commit is the only trusted Pages version marker."""

    match = PAGES_MESSAGE.fullmatch(message.strip())
    if match is None:
        raise PublicationError("Books Pages commit has no unambiguous source SHA")
    return match.group(1)


def pages_should_publish(
    candidate: str, current: str | None, comparison: str | None
) -> bool:
    """Accept only a descendant of the current books catalog's source."""

    if not SHA.fullmatch(candidate):
        raise PublicationError("Candidate source SHA is invalid")
    if current is None:
        return True
    if not SHA.fullmatch(current):
        raise PublicationError("Current Pages source SHA is invalid")
    if candidate == current:
        return False
    if comparison == "ahead":
        return True
    if comparison == "behind":
        raise PublicationError("An older run cannot replace newer Books Pages")
    if comparison == "diverged":
        raise PublicationError("A diverged run cannot replace Books Pages")
    raise PublicationError(f"Unexpected main ancestry result: {comparison!r}")


def select_release_tag(
    releases: list[dict[str, Any]], run_number: int, today: str
) -> str:
    """Reuse a run's first tag even if a rerun crosses midnight."""

    if run_number <= 0 or not re.fullmatch(r"\d{4}\.\d{2}\.\d{2}", today):
        raise PublicationError("Invalid release run number or UTC date")
    matches = []
    for release in releases:
        tag = release.get("tag_name")
        if (
            isinstance(tag, str)
            and (match := RELEASE_TAG.fullmatch(tag))
            and int(match.group(1)) == run_number
        ):
            matches.append(tag)
    if len(matches) > 1:
        raise PublicationError("Multiple book releases claim this workflow run")
    return matches[0] if matches else f"human-languages-v{today}.{run_number}"


def release_should_create(
    release: dict[str, Any] | None,
    tag: str,
    source: str,
    pdfs: list[Path],
    tag_target: str | None,
) -> bool:
    """A replay is a no-op only for byte-for-byte identical immutable assets."""

    if not RELEASE_TAG.fullmatch(tag) or not SHA.fullmatch(source):
        raise PublicationError("Invalid tag or release source SHA")
    expected = {pdf.name: pdf for pdf in pdfs}
    if (
        not expected
        or len(expected) != len(pdfs)
        or any(
            pdf.suffix != ".pdf" or pdf.is_symlink() or not pdf.is_file()
            for pdf in pdfs
        )
    ):
        raise PublicationError("Publication bundle must contain distinct PDF files")
    if release is None:
        if tag_target is not None:
            raise PublicationError("Book release tag already exists without a release")
        return True
    if tag_target != source:
        raise PublicationError("Existing release tag ref does not target this source")
    if release.get("tag_name") != tag:
        raise PublicationError("Existing release tag mismatch")
    if release.get("target_commitish") != source:
        raise PublicationError("Existing release target commit mismatch")
    if release.get("immutable") is not True:
        raise PublicationError("Existing release is not confirmed immutable")
    if release.get("draft") is True or release.get("prerelease") is True:
        raise PublicationError("Existing release is not a published book snapshot")
    assets = release.get("assets")
    if not isinstance(assets, list) or len(assets) != len(expected):
        raise PublicationError("Existing release asset set mismatch")
    seen: set[str] = set()
    for asset in assets:
        name = asset.get("name") if isinstance(asset, dict) else None
        if not isinstance(name, str) or name not in expected or name in seen:
            raise PublicationError("Existing release asset set mismatch")
        seen.add(name)
        pdf = expected[name]
        if asset.get("state") != "uploaded" or asset.get("size") != pdf.stat().st_size:
            raise PublicationError(
                f"Existing release asset size/state mismatch: {name}"
            )
        digest = "sha256:" + hashlib.sha256(pdf.read_bytes()).hexdigest()
        if asset.get("digest") != digest:
            raise PublicationError(f"Existing release asset digest mismatch: {name}")
    return False


class GitHubReader:
    """The preflight never sends a mutation request or receives a write token."""

    def __init__(self, repository: str, token: str) -> None:
        if (
            not REPOSITORY.fullmatch(repository)
            or any(part in {".", ".."} for part in repository.split("/"))
            or not token
        ):
            raise PublicationError("Repository or read token is missing/invalid")
        self.base = f"https://api.github.com/repos/{repository}/"
        self.token = token

    class _NoRedirect(HTTPRedirectHandler):
        """Do not forward the read token to a redirect destination."""

        def redirect_request(
            self,
            request: Request,
            fp: Any,
            code: int,
            msg: str,
            headers: Any,
            newurl: str,
        ) -> None:
            return None

    def get(self, path: str, *, missing_ok: bool = False) -> Any:
        request = Request(
            self.base + path,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {self.token}",
                "User-Agent": "coding-adventures-book-publication-preflight",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        try:
            with build_opener(self._NoRedirect).open(request, timeout=30) as response:
                return json.load(response)
        except HTTPError as error:
            if error.code == 404 and missing_ok:
                return None
            raise PublicationError(
                f"GitHub read API failed: HTTP {error.code}"
            ) from error
        except (URLError, ValueError) as error:
            raise PublicationError("GitHub read API failed") from error

    def pages_source(self) -> str | None:
        query = urlencode(
            {"sha": "gh-pages", "path": "human-languages/books", "per_page": 1}
        )
        commits = self.get(f"commits?{query}", missing_ok=True)
        if commits is None or commits == []:
            return None
        if not isinstance(commits, list) or len(commits) != 1:
            raise PublicationError("Cannot identify the latest Books Pages commit")
        return parse_pages_source(commits[0]["commit"]["message"])

    def comparison(self, current: str, candidate: str) -> str:
        result = self.get(f"compare/{current}...{candidate}")
        return result["status"]

    def releases(self) -> list[dict[str, Any]]:
        results: list[dict[str, Any]] = []
        for page in range(1, 21):
            batch = self.get(f"releases?per_page=100&page={page}")
            if not isinstance(batch, list):
                raise PublicationError("Release inventory is malformed")
            results.extend(batch)
            if len(batch) < 100:
                return results
        raise PublicationError(
            "Release inventory exceeded 20 pages; refusing partial search"
        )

    def run_date(self, run_id: int, run_number: int, source: str) -> str:
        if run_id <= 0 or run_number <= 0 or not SHA.fullmatch(source):
            raise PublicationError("Invalid workflow run identity")
        run = self.get(f"actions/runs/{run_id}")
        if run.get("head_sha") != source or run.get("run_number") != run_number:
            raise PublicationError("Workflow run identity does not match this source")
        created = run.get("created_at")
        if not isinstance(created, str):
            raise PublicationError("Workflow run has no creation time")
        try:
            timestamp = datetime.fromisoformat(created.replace("Z", "+00:00"))
        except ValueError as error:
            raise PublicationError("Workflow run creation time is malformed") from error
        if timestamp.tzinfo is None:
            raise PublicationError("Workflow run creation time has no timezone")
        return timestamp.astimezone(timezone.utc).strftime("%Y.%m.%d")

    def tag_target(self, tag: str) -> str | None:
        if not RELEASE_TAG.fullmatch(tag):
            raise PublicationError("Invalid book release tag")
        result = self.get(f"git/ref/tags/{quote(tag, safe='')}", missing_ok=True)
        if result is None:
            return None
        obj = result.get("object") if isinstance(result, dict) else None
        for _ in range(4):
            if not isinstance(obj, dict) or not isinstance(obj.get("sha"), str):
                raise PublicationError("Book release tag ref is malformed")
            if obj.get("type") == "commit":
                if not SHA.fullmatch(obj["sha"]):
                    raise PublicationError("Book release tag target is malformed")
                return obj["sha"]
            if obj.get("type") != "tag" or not SHA.fullmatch(obj["sha"]):
                raise PublicationError("Book release tag target is not a commit")
            annotated = self.get(f"git/tags/{obj['sha']}")
            obj = annotated.get("object") if isinstance(annotated, dict) else None
        raise PublicationError("Book release annotated tag chain is too deep")


def main() -> int:
    source = os.environ.get("COMMIT_SHA", "")
    run_number = int(os.environ.get("RUN_NUMBER", "0"))
    run_id = int(os.environ.get("RUN_ID", "0"))
    directory = Path(os.environ.get("BOOK_BUNDLE", "dist/human-languages/books"))
    pdfs = sorted(directory.glob("*.pdf"))
    if (
        not (directory / "index.html").is_file()
        or not (directory / "catalog.json").is_file()
    ):
        raise PublicationError("Downloaded book catalog is incomplete")
    api = GitHubReader(os.environ.get("REPOSITORY", ""), os.environ.get("GH_TOKEN", ""))
    current = api.pages_source()
    comparison = (
        api.comparison(current, source) if current and current != source else None
    )
    publish_pages = pages_should_publish(source, current, comparison)
    releases = api.releases()
    tag = select_release_tag(
        releases, run_number, api.run_date(run_id, run_number, source)
    )
    matching = [release for release in releases if release.get("tag_name") == tag]
    release = matching[0] if matching else None
    publish_release = release_should_create(
        release, tag, source, pdfs, api.tag_target(tag)
    )
    output = Path(os.environ["GITHUB_OUTPUT"])
    with output.open("a", encoding="utf-8") as stream:
        stream.write(f"pages={'true' if publish_pages else 'false'}\n")
        stream.write(f"release={'true' if publish_release else 'false'}\n")
        stream.write(f"tag={tag}\n")
        stream.write(
            f"name=Human Languages Books {tag.removeprefix('human-languages-')}\n"
        )
    print(
        f"Books publication preflight: Pages={publish_pages}, "
        f"release={publish_release}, tag={tag}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (PublicationError, KeyError, ValueError) as error:
        raise SystemExit(f"Books publication preflight refused: {error}") from error
