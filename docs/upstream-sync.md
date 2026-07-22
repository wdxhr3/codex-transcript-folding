# Updating the Codex Snapshot

This repository is a standalone snapshot of OpenAI Codex with a local
transcript-folding feature. It is not registered on GitHub as a fork, and
Dependabot dependency updates are not upstream Codex updates.

Update the snapshot deliberately rather than accepting independent Cargo,
Bazel, SDK, and toolchain bumps. A practical update flow is:

1. Add or refresh the upstream remote:

   ```bash
   git remote add upstream https://github.com/openai/codex.git
   git fetch upstream
   ```

2. Create a branch from the selected upstream commit:

   ```bash
   git switch -c sync/upstream-YYYY-MM-DD <upstream-commit>
   ```

3. Reapply the transcript-folding commits or port the feature as one reviewed
   patch. Resolve conflicts against the new TUI structure.

4. Run the focused transcript-folding tests and the upstream checks relevant
   to the touched code before replacing `main`.

Keeping a known-good snapshot is preferable to continuously updating unrelated
dependencies when no upstream feature or security fix is needed.
