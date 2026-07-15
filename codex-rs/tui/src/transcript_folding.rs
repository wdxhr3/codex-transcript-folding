//! UI-only transcript folding state.
//!
//! This state deliberately lives outside rollout files: folding changes how a
//! transcript is displayed, never the conversation sent to the model.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

use crate::app::App;
use crate::history_cell::AgentMarkdownCell;
use crate::history_cell::AgentMessageCell;
use crate::history_cell::HistoryCell;
use crate::history_cell::UserHistoryCell;
use crate::terminal_hyperlinks::HyperlinkLine;
use codex_protocol::ThreadId;
use codex_utils_path::write_atomically;
use ratatui::style::Stylize;
use ratatui::text::Line;
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

const FOLD_STATE_DIR: &str = "ui-state/transcript-folds";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TranscriptMessageKind {
    User,
    Assistant,
}

impl TranscriptMessageKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Assistant => "Assistant",
        }
    }
}

/// A stable-in-a-rollout identifier for a displayable conversation message.
///
/// The rollout protocol does not expose item ids on TUI history cells. The
/// message kind and its ordinal among messages of that kind are stable when a
/// session is replayed, while excluding tool and status cells that may be
/// consolidated differently during rendering.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct TranscriptMessageId {
    pub(crate) kind: TranscriptMessageKind,
    pub(crate) ordinal: usize,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct TranscriptFoldState {
    collapsed_messages: BTreeSet<TranscriptMessageId>,
}

#[derive(Default)]
pub(crate) struct TranscriptFoldCache {
    thread_id: Option<ThreadId>,
    state: TranscriptFoldState,
}

impl TranscriptFoldState {
    pub(crate) fn load(codex_home: &Path, thread_id: ThreadId) -> io::Result<Self> {
        let path = Self::path(codex_home, thread_id);
        match fs::read(path) {
            Ok(contents) => serde_json::from_slice(&contents).map_err(io::Error::other),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err),
        }
    }

    pub(crate) fn persist(&self, codex_home: &Path, thread_id: ThreadId) -> io::Result<()> {
        let path = Self::path(codex_home, thread_id);
        let contents = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        write_atomically(&path, &contents)
    }

    pub(crate) fn is_collapsed(&self, message: TranscriptMessageId) -> bool {
        self.collapsed_messages.contains(&message)
    }

    pub(crate) fn toggle(&mut self, message: TranscriptMessageId) {
        if !self.collapsed_messages.remove(&message) {
            self.collapsed_messages.insert(message);
        }
    }

    pub(crate) fn collapse_all(&mut self, messages: impl IntoIterator<Item = TranscriptMessageId>) {
        self.collapsed_messages.extend(messages);
    }

    pub(crate) fn expand_all(&mut self) {
        self.collapsed_messages.clear();
    }

    fn path(codex_home: &Path, thread_id: ThreadId) -> std::path::PathBuf {
        codex_home
            .join(FOLD_STATE_DIR)
            .join(format!("{thread_id}.json"))
    }
}

pub(crate) fn message_ids(cells: &[Arc<dyn HistoryCell>]) -> Vec<Option<TranscriptMessageId>> {
    let mut user_ordinal: usize = 0;
    let mut assistant_ordinal: usize = 0;
    let mut previous_was_assistant = false;
    cells
        .iter()
        .map(|cell| {
            if cell.as_any().is::<UserHistoryCell>() {
                previous_was_assistant = false;
                let message = TranscriptMessageId {
                    kind: TranscriptMessageKind::User,
                    ordinal: user_ordinal,
                };
                user_ordinal += 1;
                Some(message)
            } else if cell.as_any().is::<AgentMessageCell>()
                || cell.as_any().is::<AgentMarkdownCell>()
            {
                let ordinal = if previous_was_assistant {
                    assistant_ordinal.saturating_sub(1)
                } else {
                    previous_was_assistant = true;
                    let ordinal = assistant_ordinal;
                    assistant_ordinal += 1;
                    ordinal
                };
                Some(TranscriptMessageId {
                    kind: TranscriptMessageKind::Assistant,
                    ordinal,
                })
            } else {
                previous_was_assistant = false;
                None
            }
        })
        .collect()
}

pub(crate) fn is_message_start(message_ids: &[Option<TranscriptMessageId>], index: usize) -> bool {
    let Some(message) = message_ids.get(index).copied().flatten() else {
        return false;
    };
    index == 0 || message_ids[index - 1] != Some(message)
}

/// Return the normal-view replacement for a folded message cell.
///
/// A streamed assistant response may occupy several contiguous history cells.
/// All of those cells share one message id, so only the first cell emits the
/// placeholder and the remaining cells disappear from the rendered transcript.
pub(crate) fn folded_message_lines(
    message_ids: &[Option<TranscriptMessageId>],
    fold_state: &TranscriptFoldState,
    index: usize,
) -> Option<Vec<HyperlinkLine>> {
    let message = message_ids.get(index).copied().flatten()?;
    if !fold_state.is_collapsed(message) {
        return None;
    }
    if !is_message_start(message_ids, index) {
        return Some(Vec::new());
    }
    Some(vec![HyperlinkLine::new(
        Line::from(format!("▶ {} message collapsed", message.kind.label())).dim(),
    )])
}

impl App {
    fn refresh_transcript_fold_cache(&mut self) {
        let thread_id = self.chat_widget.thread_id();
        if self.transcript_fold_cache.thread_id == thread_id {
            return;
        }
        let state = thread_id
            .map(|thread_id| {
                TranscriptFoldState::load(&self.config.codex_home, thread_id).unwrap_or_else(
                    |err| {
                        warn!(%err, %thread_id, "failed to load transcript fold state");
                        TranscriptFoldState::default()
                    },
                )
            })
            .unwrap_or_default();
        self.transcript_fold_cache = TranscriptFoldCache { thread_id, state };
    }

    pub(crate) fn current_transcript_fold_state(&mut self) -> TranscriptFoldState {
        self.refresh_transcript_fold_cache();
        self.transcript_fold_cache.state.clone()
    }

    pub(crate) fn update_transcript_fold_state(&mut self, state: TranscriptFoldState) {
        self.refresh_transcript_fold_cache();
        self.transcript_fold_cache.state = state;
        let Some(thread_id) = self.transcript_fold_cache.thread_id else {
            return;
        };
        if let Err(err) = self
            .transcript_fold_cache
            .state
            .persist(&self.config.codex_home, thread_id)
        {
            warn!(%err, %thread_id, "failed to persist transcript fold state");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    fn user_cell(message: &str) -> Arc<dyn HistoryCell> {
        Arc::new(UserHistoryCell {
            message: message.to_string(),
            text_elements: Vec::new(),
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
        })
    }

    #[test]
    fn persists_folded_messages_per_thread() {
        let dir = tempdir().expect("tempdir");
        let thread_id = ThreadId::new();
        let message = TranscriptMessageId {
            kind: TranscriptMessageKind::Assistant,
            ordinal: 2,
        };
        let mut state = TranscriptFoldState::default();
        state.toggle(message);

        state
            .persist(dir.path(), thread_id)
            .expect("persist fold state");

        assert_eq!(
            TranscriptFoldState::load(dir.path(), thread_id).expect("load fold state"),
            state
        );
    }

    #[test]
    fn assigns_one_id_to_contiguous_assistant_stream_cells() {
        let cells: Vec<Arc<dyn HistoryCell>> = vec![
            user_cell("first"),
            Arc::new(AgentMessageCell::new(vec![Line::from("part one")], true)),
            Arc::new(AgentMessageCell::new(vec![Line::from("part two")], false)),
            user_cell("second"),
            Arc::new(AgentMarkdownCell::new(
                "final answer".to_string(),
                Path::new("/tmp"),
            )),
        ];

        assert_eq!(
            message_ids(&cells),
            vec![
                Some(TranscriptMessageId {
                    kind: TranscriptMessageKind::User,
                    ordinal: 0,
                }),
                Some(TranscriptMessageId {
                    kind: TranscriptMessageKind::Assistant,
                    ordinal: 0,
                }),
                Some(TranscriptMessageId {
                    kind: TranscriptMessageKind::Assistant,
                    ordinal: 0,
                }),
                Some(TranscriptMessageId {
                    kind: TranscriptMessageKind::User,
                    ordinal: 1,
                }),
                Some(TranscriptMessageId {
                    kind: TranscriptMessageKind::Assistant,
                    ordinal: 1,
                }),
            ]
        );
    }

    #[test]
    fn folded_assistant_stream_emits_one_placeholder() {
        let message = TranscriptMessageId {
            kind: TranscriptMessageKind::Assistant,
            ordinal: 0,
        };
        let ids = vec![Some(message), Some(message)];
        let mut state = TranscriptFoldState::default();
        state.toggle(message);

        let first = folded_message_lines(&ids, &state, 0).expect("folded first cell");
        let continuation = folded_message_lines(&ids, &state, 1).expect("folded continuation");

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].line.to_string(), "▶ Assistant message collapsed");
        assert!(continuation.is_empty());
    }
}
