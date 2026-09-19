#!/usr/bin/env bash
# Single source of truth for cutting a stable Desktop release tag: validate a
# `vX.Y.Z` version, refuse a duplicate, then create the annotated tag and push
# it (which fires .github/workflows/release.yml). Both release entry points call
# this so the version rule lives in exactly ONE place:
#   - `just release vX.Y.Z`                      (the laptop path)
#   - the `release` workflow's manual-dispatch step (the no-laptop path; it sets
#     a bot git identity, then calls this)
#
# `scripts/release-tag.sh --self-test` exercises the validator against a table of
# good/bad versions and makes no git changes; CI runs it (ci.yml) so the regex
# cannot silently rot. The script does NOT configure git identity (the caller's
# identity is used) and validates the version BEFORE any git command runs, so a
# malformed/hostile string can never reach `git tag`/`git push`.
set -euo pipefail

# The updater follows GitHub's stable latest release, so prerelease/build
# suffixes are deliberately rejected here and in the release workflow.
STABLE_VERSION='^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$'

# version_valid <string>: exit 0 if it matches the v+semver rule, else 1.
version_valid() {
	[[ "$1" =~ $STABLE_VERSION ]]
}

# self_test: assert the validator accepts every good case and rejects every bad
# one. Pure — touches no git state. Exits non-zero (failing CI) on any mismatch.
self_test() {
	local good=(v0.0.0 v0.1.0 v0.6.0 v1.2.3 v10.20.30)
	local bad=("" 1.2.3 v1.2 v1 vfoo v1.2.3. v01.2.3 v1.02.3 "v1.2.3 " " v1.2.3" v1.2.3-rc.1 v1.0.0+build.1 V1.2.3 $'v1.2.3\nrm -rf x')
	local rc=0 v
	for v in "${good[@]}"; do
		if ! version_valid "$v"; then
			echo "self-test FAIL: '$v' should be VALID" >&2
			rc=1
		fi
	done
	for v in "${bad[@]}"; do
		if version_valid "$v"; then
			echo "self-test FAIL: '$v' should be INVALID" >&2
			rc=1
		fi
	done
	if [[ $rc -eq 0 ]]; then
		echo "release-tag self-test: OK (${#good[@]} valid, ${#bad[@]} invalid cases)"
	fi
	return $rc
}

main() {
	if [[ "${1:-}" == "--self-test" ]]; then
		self_test
		return
	fi

	local version="${1:-}"
	if [[ -z "$version" ]]; then
		echo "usage: release-tag.sh <vX.Y.Z> | --self-test" >&2
		exit 2
	fi
	if ! version_valid "$version"; then
		echo "release: '$version' must be a stable vX.Y.Z version (for example v0.1.0)" >&2
		exit 1
	fi
	if git rev-parse -q --verify "refs/tags/$version" >/dev/null; then
		echo "release: tag '$version' already exists" >&2
		exit 1
	fi

	git tag -a "$version" -m "agent-assistant Desktop $version"
	git push origin "$version"
	echo "release: pushed $version — the Desktop release workflow will build and publish it."
}

main "$@"
