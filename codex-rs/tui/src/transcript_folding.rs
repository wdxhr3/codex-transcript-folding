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
/// session is replayed. Non-user cells between two user messages share the
/// assistant response id when that span contains assistant output.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct TranscriptMessageId {
    pub(crate) kind: TranscriptMessageKind,
    pub(crate) ordinal: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptFoldSummary {
    kind: TranscriptMessageKind,
    text_lines: usize,
    text_chars: usize,
    tool_calls: usize,
    related_items: usize,
}

impl TranscriptFoldSummary {
    pub(crate) fn label(&self) -> String {
        let noun = match self.kind {
            TranscriptMessageKind::User => "User message",
            TranscriptMessageKind::Assistant => "Assistant response",
        };
        let mut details = Vec::new();
        if self.tool_calls > 0 {
            details.push(counted(self.tool_calls, "tool call", "tool calls"));
        }
        details.push(counted(self.text_lines, "line", "lines"));
        details.push(counted(self.text_chars, "char", "chars"));
        if self.related_items > 0 {
            details.push(counted(self.related_items, "related item", "related items"));
        }
        format!("▶ {noun} collapsed · {}", details.join(" · "))
    }
}

fn counted(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
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
    let mut ids = vec![None; cells.len()];
    let mut user_ordinal: usize = 0;
    let mut assistant_ordinal: usize = 0;
    let mut index = 0;
    while index < cells.len() {
        if !cells[index].as_any().is::<UserHistoryCell>() {
            index += 1;
            continue;
        }

        ids[index] = Some(TranscriptMessageId {
            kind: TranscriptMessageKind::User,
            ordinal: user_ordinal,
        });
        user_ordinal += 1;

        let response_start = index + 1;
        let mut response_end = response_start;
        while response_end < cells.len() && !cells[response_end].as_any().is::<UserHistoryCell>() {
            response_end += 1;
        }
        if cells[response_start..response_end]
            .iter()
            .any(|cell| is_assistant_message_cell(cell.as_ref()))
        {
            let response = TranscriptMessageId {
                kind: TranscriptMessageKind::Assistant,
                ordinal: assistant_ordinal,
            };
            assistant_ordinal += 1;
            ids[response_start..response_end].fill(Some(response));
        }
        index = response_end;
    }
    ids
}

fn is_assistant_message_cell(cell: &dyn HistoryCell) -> bool {
    cell.as_any().is::<AgentMessageCell>() || cell.as_any().is::<AgentMarkdownCell>()
}

pub(crate) fn is_message_start(message_ids: &[Option<TranscriptMessageId>], index: usize) -> bool {
    let Some(message) = message_ids.get(index).copied().flatten() else {
        return false;
    };
    index == 0 || message_ids[index - 1] != Some(message)
}

pub(crate) fn is_selectable_message_start(
    cells: &[Arc<dyn HistoryCell>],
    message_ids: &[Option<TranscriptMessageId>],
    index: usize,
) -> bool {
    let Some(message) = message_ids.get(index).copied().flatten() else {
        return false;
    };
    let is_matching_message_cell = match message.kind {
        TranscriptMessageKind::User => cells[index].as_any().is::<UserHistoryCell>(),
        TranscriptMessageKind::Assistant => is_assistant_message_cell(cells[index].as_ref()),
    };
    is_matching_message_cell
        && !(0..index).any(|earlier| {
            message_ids[earlier] == Some(message)
                && match message.kind {
                    TranscriptMessageKind::User => cells[earlier].as_any().is::<UserHistoryCell>(),
                    TranscriptMessageKind::Assistant => {
                        is_assistant_message_cell(cells[earlier].as_ref())
                    }
                }
        })
}

pub(crate) fn message_summaries(
    cells: &[Arc<dyn HistoryCell>],
    message_ids: &[Option<TranscriptMessageId>],
) -> Vec<Option<TranscriptFoldSummary>> {
    let mut summaries = vec![None; cells.len()];
    for (index, message) in message_ids.iter().copied().enumerate() {
        let Some(message) = message else {
            continue;
        };
        if !is_message_start(message_ids, index) {
            continue;
        }

        let mut summary = TranscriptFoldSummary {
            kind: message.kind,
            text_lines: 0,
            text_chars: 0,
            tool_calls: 0,
            related_items: 0,
        };
        for (cell, cell_message) in cells.iter().zip(message_ids) {
            if *cell_message != Some(message) {
                continue;
            }
            let is_message_cell = match message.kind {
                TranscriptMessageKind::User => cell.as_any().is::<UserHistoryCell>(),
                TranscriptMessageKind::Assistant => is_assistant_message_cell(cell.as_ref()),
            };
            if is_message_cell {
                let lines = cell.raw_lines();
                summary.text_lines += lines.len();
                summary.text_chars += lines
                    .iter()
                    .map(|line| line.to_string().chars().count())
                    .sum::<usize>();
            } else {
                let tool_calls = cell.transcript_tool_call_count();
                summary.tool_calls += tool_calls;
                if tool_calls == 0 {
                    summary.related_items += 1;
                }
            }
        }
        summaries[index] = Some(summary);
    }
    summaries
}

/// Return the normal-view replacement for a folded message cell.
///
/// An assistant response may occupy several message and activity cells. All of
/// those cells share one message id, so only the first cell emits the summary
/// placeholder and the remaining cells disappear from the rendered transcript.
pub(crate) fn folded_message_lines(
    message_ids: &[Option<TranscriptMessageId>],
    message_summaries: &[Option<TranscriptFoldSummary>],
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
    let label = message_summaries
        .get(index)
        .and_then(Option::as_ref)
        .map(TranscriptFoldSummary::label)
        .unwrap_or_else(|| format!("▶ {} message collapsed", message.kind.label()));
    Some(vec![HyperlinkLine::new(Line::from(label).dim())])
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
        let cells: Vec<Arc<dyn HistoryCell>> = vec![
            Arc::new(AgentMessageCell::new(vec![Line::from("part one")], true)),
            Arc::new(AgentMessageCell::new(vec![Line::from("part two")], false)),
        ];
        let ids = vec![Some(message), Some(message)];
        let summaries = message_summaries(&cells, &ids);
        let mut state = TranscriptFoldState::default();
        state.toggle(message);

        let first = folded_message_lines(&ids, &summaries, &state, 0).expect("folded first cell");
        let continuation =
            folded_message_lines(&ids, &summaries, &state, 1).expect("folded continuation");

        assert_eq!(first.len(), 1);
        assert_eq!(
            first[0].line.to_string(),
            "▶ Assistant response collapsed · 2 lines · 16 chars"
        );
        assert!(continuation.is_empty());
    }

    #[test]
    fn assigns_tool_activity_to_the_surrounding_assistant_response() {
        let cells: Vec<Arc<dyn HistoryCell>> = vec![
            user_cell("look this up"),
            Arc::new(AgentMessageCell::new(
                vec![Line::from("I will search")],
                false,
            )),
            Arc::new(crate::history_cell::new_web_search_call(
                "call-1".to_string(),
                "Codex".to_string(),
                codex_app_server_protocol::WebSearchAction::Other,
            )),
            Arc::new(AgentMarkdownCell::new(
                "final answer".to_string(),
                Path::new("/tmp"),
            )),
        ];
        let ids = message_ids(&cells);
        let assistant = TranscriptMessageId {
            kind: TranscriptMessageKind::Assistant,
            ordinal: 0,
        };

        assert_eq!(
            ids[1..],
            [Some(assistant), Some(assistant), Some(assistant)]
        );
        assert!(is_selectable_message_start(&cells, &ids, 1));
        assert!(!is_selectable_message_start(&cells, &ids, 2));
        assert!(!is_selectable_message_start(&cells, &ids, 3));

        let summaries = message_summaries(&cells, &ids);
        assert_eq!(
            summaries[1].as_ref().map(TranscriptFoldSummary::label),
            Some("▶ Assistant response collapsed · 1 tool call · 2 lines · 25 chars".to_string())
        );
    }
}
