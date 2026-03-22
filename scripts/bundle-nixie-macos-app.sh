#!/usr/bin/env bash
# 将 release 版 nixie-pet 打成 Codepet.app（Dock 显示图标；双击不经过终端）。
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="${ROOT}/target/release/nixie-pet"
ICNS="${ROOT}/nixie-pet/assets/icon/AppIcon.icns"
PLIST_SRC="${ROOT}/nixie-pet/packaging/macos/Info.plist"
OUT_DIR="${1:-"${ROOT}/dist"}"
APP="${OUT_DIR}/Codepet.app"

if [[ ! -f "${ICNS}" ]]; then
  echo "缺少 ${ICNS}。在 macOS 上执行: python3 nixie-pet/assets/icon/build_macos_icon.py" >&2
  exit 1
fi
if [[ ! -x "${EXE}" ]]; then
  echo "请先编译: cargo build -p nixie-pet --release" >&2
  exit 1
fi

VERSION="$(grep -E '^version\s*=' "${ROOT}/nixie-pet/Cargo.toml" | head -1 | sed -E 's/^version\s*=\s*"([^"]+)".*/\1/')"
mkdir -p "${OUT_DIR}"
rm -rf "${APP}"
mkdir -p "${APP}/Contents/MacOS" "${APP}/Contents/Resources"
cp "${EXE}" "${APP}/Contents/MacOS/codepet"
chmod +x "${APP}/Contents/MacOS/codepet"
cp "${ICNS}" "${APP}/Contents/Resources/AppIcon.icns"
cp "${PLIST_SRC}" "${APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${VERSION}" "${APP}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}" "${APP}/Contents/Info.plist"

echo "Built ${APP}"
echo "启动: open \"${APP}\""
