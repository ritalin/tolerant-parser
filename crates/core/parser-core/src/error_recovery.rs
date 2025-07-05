use std::collections::HashSet;

use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use scanner_core::iter::LookaheadIterator;
use stitch_handler::StitchRecoveryHandler;
use crate::state_stack::StateStack;

pub mod delete_recovery;
pub mod shift_recovery;
pub mod stitch_handler;

pub struct RecoveryEventDispatcher {
    engine: ParsingRuleSet,
    penalty: RecoveryPenalty,
    except_kind_ids: HashSet<u32>,
}

impl RecoveryEventDispatcher {
    pub fn new(penalty: RecoveryPenalty, except_kinds: &[SyntaxKind], engine: ParsingRuleSet) -> Self {
        Self { penalty, engine, except_kind_ids: HashSet::from_iter(except_kinds.iter().map(|kind| kind.id)) }
    }

    #[cfg(feature = "test_support")]
    #[doc(hidden)]
    pub fn handle_from_history(&mut self, state_histories: &[usize], lookaheads: LookaheadIterator) -> Option<Vec<RecoveryEvent>> {
        let state_stack = make_stack(state_histories);
        self.handle(&state_stack, lookaheads)
    }

    pub(crate) fn handle(&mut self, state_stack: &StateStack, lookaheads: LookaheadIterator) -> Option<Vec<RecoveryEvent>> {
        let Some(lookahead) = lookaheads.peek() else {
            return None;
        };

        let mut delete_recovery = delete_recovery::DeleteErrorRecovery::new_with_stack(state_stack.clone(), self.penalty.clone(), self.engine);
        let mut shift_recovery = shift_recovery::ShiftErrorRecovery::new_with_stack(state_stack.clone(), self.penalty.clone(), &self.except_kind_ids, self.engine);
        let stitch_handler = StitchRecoveryHandler::new(self.engine);

        let mut report = None;

        // try a shift error recovery
        while let Some(candidate) = shift_recovery.handle(lookahead) {
            let next_report = stitch_handler.try_recovery(candidate, lookaheads.clone());
            match (report.as_ref(), next_report.as_ref()) {
                (None, Some(_)) => {
                    report = next_report;
                }
                (Some(lhs), Some(rhs)) if rhs.judge_score(lhs) => {
                    report = next_report;
                }
                _ => {}
            }
        }

        if let Some(report) = report.as_ref() {
            self.penalty.accept_shift();
            return Some(report.events());
        }

        // try a delete error recovery
        report = delete_recovery.handle(lookaheads.clone()).and_then(|candidate| {
            stitch_handler.try_recovery(candidate, lookaheads.clone().skip(self.penalty.delete_slot - delete_recovery.left_slot()))
        });

        if let Some(report) = report.as_ref() {
            self.penalty.accept_delete(delete_recovery.left_slot());
            return Some(report.events());
        }

        None
    }

    pub fn handle_as_invalid(&self, lookaheads: LookaheadIterator) -> Vec<RecoveryEvent> {
        let full_emit_symbol = self.engine.full_emit_config().to_symbol;
        let mut invalids = Vec::with_capacity(lookaheads.len());

        let mut iter = lookaheads.filter(|la| la.main.kind != full_emit_symbol).peekable();

        while let Some(la) = iter.next() {
            invalids.push(RecoveryEvent::Invalid { kind: la.main.kind, need_emit: iter.peek().is_none() });
        }

        invalids
    }

    pub fn penalty(&self) -> RecoveryPenalty {
        self.penalty.clone()
    }
}

#[cfg(feature = "test_support")]
#[doc(hidden)]
fn make_stack(state_histories: &[usize]) -> StateStack {
    let initial_state = state_histories.first().cloned().unwrap_or_default();

    let mut stack = StateStack::new(initial_state);

    for state in state_histories.iter() {
        stack.push_state(*state);
    }

    stack
}

#[derive(Clone, Debug)]
pub struct RecoveryPenalty {
    /// Max delete-recovery challenging
    pub delete_slot: usize,
    /// Max shift-recovery depth
    pub shift_limit: usize,
    /// Current shift-recovery penalty
    pub shift_decay: usize,
    /// Next shift-recovery penalty
    pub next_shift_decay: usize,
    /// Max shift-recovery candidate packet size
    pub max_shift_packet_size: usize,
}

impl RecoveryPenalty {
    pub fn accept_delete(&mut self, left_slot: usize) {
        self.delete_slot = left_slot;
    }
    pub fn accept_shift(&mut self) {
        self.shift_decay = self.next_shift_decay;
        self.next_shift_decay <<= 1;
    }
}

impl Default for RecoveryPenalty {
    fn default() -> Self {
        Self { 
            delete_slot: 3,
            shift_limit: 10,
            shift_decay: 0,
            next_shift_decay: 1,
            max_shift_packet_size: 10,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct RecoveryReport {
    events: cactus::Cactus<(u32, RecoveryEvent)>,
    state_stack: StateStack,
    patch_score: usize,
    stitch_score: usize,
    depth: usize,
}

impl RecoveryReport {
    #[cfg(feature = "test_support")]
    #[doc(hidden)]
    pub fn new(state_histories: &[usize]) -> Self {
        Self::new_with_stack(make_stack(state_histories))
    }

    pub(crate) fn new_with_stack(state_stack: StateStack) -> Self {
        Self {
            events: cactus::Cactus::new(),
            state_stack,
            patch_score: 0,
            stitch_score: 0,
            depth: 0,
        }
    }

    pub fn next_report(&self) -> Self {
        let mut report = self.clone();
        report.depth += 1;
        report
    }

    pub fn top_state(&self) -> Option<usize> {
        self.state_stack.peek_state().cloned()
    }

    pub fn push_event(&mut self, kind_id: u32, event: RecoveryEvent) {
        self.events = self.events.child((kind_id, event));
    }

    pub fn events(&self) -> Vec<RecoveryEvent> {
        let event_len = self.events.len();
        let mut events = Vec::with_capacity(event_len);

        for (_, event) in self.events.vals().cloned() {
            events.push(event);
        }
        events.reverse();
        events
    }

    pub fn contains_kind(&self, kind: SyntaxKind) -> bool {
        self.events.vals().any(|(id, _)| *id == kind.id)
    }

    #[inline]
    pub fn patch_score(&self) -> usize {
        self.patch_score
    }
    #[inline]
    pub fn stitch_score(&self) -> usize {
        self.stitch_score
    }

    pub fn judge_score(&self, other: &Self) -> bool {
        match self.stitch_score.cmp(&other.stitch_score) {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }

        self.patch_score < other.patch_score
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum RecoveryEvent {
    /// A patch deleting action that was accepted during recovery.
    PatchDelete{ kind: SyntaxKind, state: usize },
    /// A patch shifting action that was accepted during recovery.
    PatchShift(RecoveryEventPayload),
    /// A normal parsing transition that occurred after patching.
    Stitch(RecoveryEventPayload),
    /// A fallback event indicating recovery failed with no viable path.
    Invalid { kind: SyntaxKind, need_emit: bool },
}

#[derive(PartialEq, Clone, Debug)]
pub enum RecoveryEventPayload {
    /// A Shift action performed during patch/stitch phase.
    Shift { kind: SyntaxKind, state: usize, next_state: usize },
    /// A Reduce action performed during patch/stitch phase.
    Reduce{ kind: SyntaxKind, state: usize, next_state: usize, pop_count: usize, },
    /// A Accept action performed during patch/stitch phase.
    Accept{ kind: SyntaxKind, last_state: usize },
}