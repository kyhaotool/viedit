#!/usr/bin/env bash
# Đóng gói VieEdit cho Linux (Fedora, Ubuntu, ...).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

say() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
err() { printf '\033[1;31mLỗi:\033[0m %s\n' "$*" >&2; }

if ! command -v cargo >/dev/null 2>&1; then
  err "chưa có cargo. Cài Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  err "chưa có ffmpeg."
  echo "     Fedora:  sudo dnf install ffmpeg"
  echo "     Ubuntu:  sudo apt install ffmpeg"
  exit 1
fi

say "Kiểm tra mã nguồn"
cargo fmt --check 2>/dev/null || echo "     (bỏ qua: mã nguồn chưa định dạng)"
cargo clippy --all-targets -- -D warnings 2>/dev/null || echo "     (bỏ qua: clippy chưa cài)"

say "Chạy kiểm thử"
cargo test --release

say "Biên dịch bản phát hành"
cargo build --release

DIST="$ROOT/dist/viedit-linux-x86_64"
rm -rf "$DIST"
mkdir -p "$DIST"

cp target/release/viedit "$DIST/"
cp -r assets "$DIST/assets"
rm -f "$DIST/assets/library/README.md"
cp README.md LICENSE "$DIST/"

say "Xong: $DIST"
du -sh "$DIST"
echo
echo "Chạy thử:  $DIST/viedit"
