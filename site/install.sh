#!/bin/sh
# Installs evoke from its GitHub releases:
#
#   curl -fsSL https://evoke.build/install.sh | sh
#
# Fetches the archive for this machine and the release's SHA256SUMS, checks the one against the other, and puts
# `evoke` in ~/.local/bin — or in EVOKE_INSTALL. EVOKE_VERSION picks a version, X.Y.Z; unset, the latest release.
set -eu

repo="https://github.com/evoke-build/evoke"
dir=${EVOKE_INSTALL:-$HOME/.local/bin}

fail() { echo "install.sh: $1" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "$1 is needed and is not on PATH"; }
need curl
need tar

case $(uname -s) in
  Darwin) os=apple-darwin ;;
  Linux) os=unknown-linux-musl ;;
  *) fail "evoke runs on macOS and Linux — Windows through WSL — not on $(uname -s)" ;;
esac
case $(uname -m) in
  arm64 | aarch64) arch=aarch64 ;;
  x86_64 | amd64) arch=x86_64 ;;
  *) fail "no build for $(uname -m); the releases are at $repo/releases" ;;
esac
archive="evoke-$arch-$os.tar.gz"
release="$repo/releases/latest/download"
[ -z "${EVOKE_VERSION:-}" ] || release="$repo/releases/download/v${EVOKE_VERSION#v}"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fetch() { curl -fsSL --proto '=https' --tlsv1.2 --output "$tmp/$1" "$release/$1" || fail "$release/$1 could not be fetched"; }
fetch "$archive"
fetch SHA256SUMS

expected=$(awk -v file="$archive" '$2 == file { print $1 }' "$tmp/SHA256SUMS")
[ -n "$expected" ] || fail "SHA256SUMS does not list $archive"
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/$archive" | cut -d ' ' -f 1)
else
  actual=$(shasum -a 256 "$tmp/$archive" | cut -d ' ' -f 1)
fi
[ "$actual" = "$expected" ] || fail "$archive does not match SHA256SUMS; run this again, and report it if it happens twice"

tar -xzf "$tmp/$archive" -C "$tmp" evoke
mkdir -p "$dir"
# Into place by rename, so a running `evoke` is replaced whole and never overwritten under itself.
install -m 755 "$tmp/evoke" "$dir/.evoke.new" && mv -f "$dir/.evoke.new" "$dir/evoke"

case $dir in
  "$HOME"/*) shown="~${dir#"$HOME"}" ;;
  *) shown=$dir ;;
esac
echo "installed $("$dir/evoke" --version) as $shown/evoke"
case ":$PATH:" in
  *":$dir:"*) ;;
  *) echo "  add it to your PATH  →  export PATH=\"$dir:\$PATH\"" ;;
esac
echo "  next: the first ten minutes  →  https://evoke.build/manual/start/first-run.html"
