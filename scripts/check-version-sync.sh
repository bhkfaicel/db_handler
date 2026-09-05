#!/usr/bin/env bash
# Fail if Cargo.toml [package].version is out of sync with CHANGELOG.md.
set -euo pipefail

if [[ -n "${CRATE_ROOT:-}" ]]; then
  ROOT="$(cd "${CRATE_ROOT}" && pwd)"
else
  ROOT="$(cd "$(dirname "$0")/.." && pwd)"
fi

CARGO_TOML="${ROOT}/Cargo.toml"
CHANGELOG="${ROOT}/CHANGELOG.md"

if [[ ! -f "${CARGO_TOML}" ]]; then
  echo "error: missing ${CARGO_TOML}" >&2
  exit 1
fi
if [[ ! -f "${CHANGELOG}" ]]; then
  echo "error: missing ${CHANGELOG}" >&2
  exit 1
fi

CARGO_VERSION="$(
  sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "${CARGO_TOML}" | head -n 1
)"
if [[ -z "${CARGO_VERSION}" ]]; then
  echo "error: could not read version from ${CARGO_TOML}" >&2
  exit 1
fi

if [[ "${CARGO_VERSION}" == *-rc* ]]; then
  echo "error: Cargo.toml version '${CARGO_VERSION}' must be a stable base (no '-rc');" >&2
  exit 1
fi

FIRST_HEADING="$(
  sed -n 's/^##[[:space:]]*\[\([^]]*\)\].*/\1/p' "${CHANGELOG}" | head -n 1
)"
RELEASED_VERSION="$(
  sed -n 's/^##[[:space:]]*\[\([0-9][^]]*\)\].*/\1/p' "${CHANGELOG}" | head -n 1
)"
if [[ -z "${RELEASED_VERSION}" ]]; then
  echo "error: could not read a released '## [x.y.z]' heading from ${CHANGELOG}" >&2
  exit 1
fi

LOWER_HEADING="$(printf '%s' "${FIRST_HEADING}" | tr '[:upper:]' '[:lower:]')"
if [[ "${LOWER_HEADING}" == "unreleased" ]]; then
  echo "version sync OK: ${CARGO_VERSION} pending release"
  exit 0
fi

if [[ "${CARGO_VERSION}" != "${RELEASED_VERSION}" ]]; then
  echo "error: version mismatch Cargo.toml=${CARGO_VERSION} CHANGELOG=${RELEASED_VERSION}" >&2
  exit 1
fi

echo "version sync OK: ${CARGO_VERSION}"
