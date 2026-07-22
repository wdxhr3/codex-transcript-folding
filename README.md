# Codex Transcript Folding

[Chinese](README.zh-CN.md) | [English](README.en.md)

An experimental Codex CLI fork that adds user-controlled, UI-only folding for
completed user and assistant messages. Fold choices are saved per task, restored
by `codex resume`, and reflected in both the `Ctrl+T` transcript and the normal
inline interface.

This fork does not automatically fold content and does not remove anything from
the model context, stored conversation, or workspace.

- [English documentation](README.en.md)
- [Chinese documentation](README.zh-CN.md)
- [Detailed English design and verification guide](codex-rs/tui/TRANSCRIPT_FOLDING.en.md)
- [Detailed Chinese design and verification guide](codex-rs/tui/TRANSCRIPT_FOLDING.zh-CN.md)
- [Upstream snapshot update strategy](docs/upstream-sync.md)
- [Upstream Codex repository](https://github.com/openai/codex)

Licensed under [Apache-2.0](LICENSE). See [NOTICE](NOTICE) for attribution.
