# Workflow Strategy

This repository is an experimental Codex CLI snapshot, not the upstream
OpenAI release repository. Its CI intentionally verifies only the maintained
transcript-folding feature.

`ci.yml` runs on pull requests, pushes to `main`, and manual dispatch. It
uses a standard Ubuntu runner to check Rust formatting and the focused
`codex-tui` regression tests for transcript folding, transcript rendering,
and resize reflow.

Upstream release, CLA, issue-management, Bazel matrix, SDK, and scheduled
workflows are intentionally omitted. Those workflows depend on upstream
infrastructure and create misleading failures or unnecessary scheduled runs
in this standalone repository.
