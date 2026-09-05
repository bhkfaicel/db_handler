#!/usr/bin/env bash
set -euo pipefail
VERSION="${1:?usage: release-branch-name.sh <version>}"
if [[ "$VERSION" == *-rc* ]]; then
  echo "error: expected a stable version (no -rc): got '${VERSION}'" >&2
  exit 1
fi
if [[ ! "$VERSION" =~ ^([0-9]+)\.([0-9]+)(\.[0-9]+)?$ ]]; then
  echo "error: version '${VERSION}' is not X.Y or X.Y.Z" >&2
  exit 1
fi
echo "release/${BASH_REMATCH[1]}.${BASH_REMATCH[2]}"
