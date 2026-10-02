# shellcheck shell=bash
# npm-lock.sh -- install an emitted Engram project from a committed lockfile.
#
# Sourced by build-web.sh and build-electron.sh; not run on its own.
#
# ## Why a committed lockfile
#
# The Mosaic emitter writes package.json for the web and desktop projects with
# exact versions for every DIRECT dependency, but nothing pins what those
# dependencies pull in. Vite, Electron and electron-builder bring several
# hundred transitive packages between them, and `npm install` resolves each one
# fresh, to whatever the registry serves that day. Those packages run at build
# time (and Electron's run inside the shipped app), so a release built that way
# contains code nobody chose and nobody can reproduce.
#
# A lockfile fixes every package to one version and one integrity hash, and
# `npm ci` installs exactly that or fails. The emitted project is regenerated
# from scratch on every build, so it cannot carry its own lockfile; the lock
# lives in this package instead, one per emitted project:
#
#     npm/web/package-lock.json        <- build-web.sh       (emitted react/)
#     npm/electron/package-lock.json   <- build-electron.sh  (emitted electron/)
#
# ## When the lock and package.json disagree
#
# `npm ci` refuses to install if package.json names a dependency or version the
# lock does not have -- which is what happens when the emitter bumps a version.
# That refusal is the point: a dependency change becomes a reviewed lockfile
# diff instead of a silent drift. Regenerate with the script's --update-lock
# flag and commit the result.

# install_from_lock <emitted-project-dir> <committed-lockfile>
install_from_lock() {
  local app="$1" lock="$2"
  if [[ ! -f "$lock" ]]; then
    echo "error: no committed lockfile at $lock" >&2
    echo "       generate it with --update-lock and commit it" >&2
    return 1
  fi
  cp "$lock" "$app/package-lock.json"
  if ! ( cd "$app" && npm ci --no-audit --no-fund ); then
    echo "error: npm ci could not install from $lock" >&2
    echo "       if the emitted package.json changed its dependencies, regenerate" >&2
    echo "       the lock with --update-lock, review the diff, and commit it" >&2
    return 1
  fi
}

# update_lock <emitted-project-dir> <committed-lockfile>
#
# Resolves the emitted package.json into a fresh lockfile without installing
# anything (--package-lock-only) or running any package's install scripts, then
# copies it over the committed one.
update_lock() {
  local app="$1" lock="$2"
  rm -f "$app/package-lock.json"
  ( cd "$app" && npm install --package-lock-only --ignore-scripts --no-audit --no-fund )
  mkdir -p "$(dirname "$lock")"
  cp "$app/package-lock.json" "$lock"
  echo "Updated: $lock (review the diff before committing)"
}
