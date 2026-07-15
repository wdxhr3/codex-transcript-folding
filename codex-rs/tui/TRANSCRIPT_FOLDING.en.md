# Codex CLI Transcript Folding: Usage, Design, and Manual Verification

[中文](TRANSCRIPT_FOLDING.zh-CN.md) | [English](TRANSCRIPT_FOLDING.en.md) | [Project documentation](../../README.en.md)

## 1. Feature boundary

This change adds UI-only transcript folding to Codex CLI:

- individually fold any completed user or assistant message;
- fold all user and assistant messages;
- expand all messages;
- persist fold state per Codex task/thread;
- preserve fold state when the task is restored with `codex resume`;
- rebuild the normal inline history with placeholders when the `Ctrl+T`
  transcript closes.

Folding does not modify the rollout, model context, tool calls, file changes, or
workspace. It changes only terminal rendering and never runs automatically.

## 2. Source map and state format

- Upstream source: <https://github.com/openai/codex>
- This repository: <https://github.com/wdxhr3/codex-transcript-folding>
- Rust workspace: `codex-rs/`
- Fold state and message IDs: `codex-rs/tui/src/transcript_folding.rs`
- `Ctrl+T` selection and actions: `codex-rs/tui/src/pager_overlay.rs`
- Overlay open/close and inline repaint: `codex-rs/tui/src/app_backtrack.rs`
- Normal-history replay: `codex-rs/tui/src/app/resize_reflow.rs`
- Persistent state: `~/.codex/ui-state/transcript-folds/<thread-id>.json`

Example state:

```json
{
  "collapsed_messages": [
    { "kind": "user", "ordinal": 0 },
    { "kind": "assistant", "ordinal": 1 }
  ]
}
```

The ordinal is the message's order among messages of the same kind within the
task. Tool output, status cards, and reasoning cells are neither numbered nor
folded. Files are written atomically.

## 3. Why selection happens in `Ctrl+T`

Codex CLI's normal inline interface writes completed messages into native
terminal scrollback. Once written, Codex no longer owns a component tree in
which every old message can be selected reliably.

The `Ctrl+T` transcript is a full-screen view managed by Codex and retains the
boundaries of every history cell. It is therefore the safe place to select
messages. When it closes, Codex discards its managed history presentation and
replays the original cells. Folded messages become placeholders while other
content is rendered normally.

The resulting flow is:

1. press `Ctrl+T` in the normal interface;
2. choose and fold messages in the transcript;
3. close the transcript;
4. see the same result immediately in the normal interface.

Selecting old messages directly inside native scrollback would require a much
larger architecture change: replacing inline scrollback with a transcript that
Codex owns and renders continuously.

## 4. Shortcuts

| Key | Action |
| --- | --- |
| `Tab` | Select the next user or assistant message |
| `Shift+Tab` | Select the previous user or assistant message |
| `Space` | Toggle the selected message |
| `f` | Fold all user and assistant messages |
| `Shift+F` | Expand all messages |
| `q` or `Ctrl+T` | Close and repaint the normal interface |

A streamed assistant response may occupy several internal cells. It is treated
as one message and produces only one `▶ Assistant message collapsed`
placeholder.

## 5. Build and run

The repository pins Rust and Cargo 1.95.0 in
`codex-rs/rust-toolchain.toml`, including `rustfmt`, `clippy`, and `rust-src`.
Developer checks also use `just`, `dotslash`, and `cargo-nextest`; verified
versions were just 1.56.0, DotSlash 0.5.9, and cargo-nextest 0.9.140.

macOS requires Xcode Command Line Tools. Ubuntu/Debian requires a C/C++
toolchain, `pkg-config`, and `libcap-dev`. Cargo fetches dependencies and
submodules pinned by `codex-rs/Cargo.lock`. If Google Source is blocked, an
administrator-approved `libyuv` mirror may be used only if the locked commit is
preserved.

```shell
git clone https://github.com/wdxhr3/codex-transcript-folding.git
cd codex-transcript-folding
just fmt-check
just clippy -p codex-tui -- -D warnings
just test -p codex-tui
```

Build and start this fork:

```shell
cd codex-rs
cargo build --locked --bin codex
cargo run --locked --bin codex -- --no-alt-screen
```

After the first build:

```shell
./target/debug/codex --no-alt-screen
./target/debug/codex resume --last --no-alt-screen
./target/debug/codex resume --no-alt-screen
```

## 6. Manual verification

### A. Fold one user message

1. Start this fork and send at least two distinguishable prompts.
2. Wait for the assistant replies to complete, then press `Ctrl+T`.
3. Press `Tab`; selection should move only among user and assistant messages.
4. Select the first user message and press `Space`.
5. Expect `▶ User message collapsed` in the transcript.
6. Press `q` and expect the same placeholder in the normal interface.

### B. Fold one assistant message

1. Open `Ctrl+T` again and select a completed assistant response.
2. Press `Space`.
3. Expect exactly one `▶ Assistant message collapsed`, even if the original
   response arrived through several streamed cells.
4. Press `q` and confirm the normal interface matches.

### C. Fold all and expand all

1. Open `Ctrl+T`, press lowercase `f`, and expect all user/assistant messages to
   become placeholders. Tool and status cells remain visible.
2. Close and confirm the normal interface matches.
3. Reopen, press `Shift+F`, and expect every original message to return.
4. Close and confirm the normal interface is expanded too.

### D. Resume persistence

1. Fold at least one user and one assistant message, then close the transcript.
2. Exit Codex normally.
3. Run `./target/debug/codex resume --last --no-alt-screen`.
4. After replay, expect the same messages to remain folded in the normal view.
5. Open `Ctrl+T` and confirm its state matches.
6. Confirm a matching JSON file exists under
   `~/.codex/ui-state/transcript-folds/`.

### E. Semantics remain unchanged

1. Fold an old message that contains a memorable fact.
2. Ask a follow-up that depends on that fact; Codex should still have the full
   context.
3. Inspect `git status` or `git diff`; the fold action itself must not change
   project files.
4. Expand the message and confirm the complete original text returns.

### F. `/clear` does not leak ordinals

1. Fold the first user message.
2. Run `/clear`.
3. Send a new message.
4. Confirm the new ordinal-zero message does not inherit the old fold state.

## 7. Known behavior and limitations

- Only completed history can be folded; an active streaming cell cannot be
  selected.
- Rebuilding managed history may produce a small flicker and may reset terminal
  text selection or scroll position.
- Fold state is independent of rollout data; copying only a rollout file does
  not copy its UI state.
- `/clear` clears the current ordinal mapping.
- A fork has a new thread ID and does not inherit the parent's fold state.
- There is no automatic folding.

## 8. Safety boundary

Folding does not:

- remove content from the model context;
- delete stored conversation history;
- undo commands, file changes, or external actions;
- modify Git history;
- change instructions that the agent follows.

It is reversible display organization, not deletion, forgetting, or rollback.
