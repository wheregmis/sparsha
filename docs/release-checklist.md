# Release Checklist

Use this checklist when preparing the Sparsha 1.0 candidate or a later tagged release.

## Automated Gates

Run or confirm all of the following:

1. GitHub Actions `CI` workflow is green on the target commit.
2. GitHub Actions `Release Readiness` workflow is green on the target commit.
3. GitHub Actions `Showcase Pages` workflow is green if the public demo should be refreshed from this commit.
4. Review uploaded artifacts from `Release Readiness`:
   - `playwright-report`
   - `test-results`
   - `artifacts/web-smoke`
   - `artifacts/perf`
   - `artifacts/lighthouse`
5. Local fallback entrypoint remains available:
   - `./scripts/release-readiness.sh`

## Manual Sign-Off

Complete the manual checks that are still intentionally outside the automated gate:

1. Accessibility smoke verification for the built-in widgets on native and web.
2. Browser/native parity smoke verification for the core interaction flows that matter for 1.0.
3. Review the examples that exercise the main product shapes:
   - `kitchen-sink`
   - `fractal-clock`
   - `hybrid-overlay`
   - `showcase`
   - `todo`

## Documentation And Release Notes

Before tagging a release:

1. Update [CHANGELOG.md](/Users/wheregmis/Documents/GitHub/spark/CHANGELOG.md) from `Unreleased` into the pending release section.
2. Confirm [README.md](/Users/wheregmis/Documents/GitHub/spark/README.md), [examples/README.md](/Users/wheregmis/Documents/GitHub/spark/examples/README.md), and [docs/api-surface.md](/Users/wheregmis/Documents/GitHub/spark/docs/api-surface.md) still describe the shipped 1.0 surface accurately.
3. Confirm the supported `TaskRuntime` story still matches the shipped executor behavior:
   - built-in task kinds: `echo`, `sleep_echo`, `analyze_text`
   - no custom task registration in the 1.0 contract
4. Confirm the platform/runtime dependency pin rationale is still accurate:
   - `winit = 0.30.12` is intentionally pinned to the current stable line
   - the pin should continue to align with `ui-events-winit` and `accesskit_winit`

## Release Manager Steps

1. Bump versions if the release requires it.
2. Commit changelog and release-note updates.
3. Create and push the release tag.
4. Publish the GitHub release entry with the finalized notes.
