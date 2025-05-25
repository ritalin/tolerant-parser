use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use scanner_core::Token;
use stitch_handler::StitchRecoveryHandler;
use crate::{state_stack::StateStack, Recovery};

pub mod delete_recovery;
pub mod shift_recovery;
pub mod stitch_handler;

pub struct PatchEventDispatcher {
    engine: ParsingRuleSet,
    penalty: RecoveryPenalty,
}

impl PatchEventDispatcher {
    pub fn new(penalty: RecoveryPenalty, engine: ParsingRuleSet) -> Self {
        Self { penalty, engine }
    }

    pub(crate) fn handle(&mut self, failed_state: usize, state_stack: StateStack, lookaheads: std::slice::Iter<Token>) -> Option<Vec<RecoveryEvent>> {
        let mut peekable = lookaheads.clone().peekable();
        let Some(lookahead) = peekable.peek() else {
            return None;
        };

        let mut delete_recovery = delete_recovery::DeleteErrorRecovery::new_with_stack(failed_state, state_stack.clone(), self.penalty.clone(), self.engine);
        let mut shift_recovery = shift_recovery::ShiftErrorRecovery::new_with_stack(failed_state, state_stack.clone(), self.penalty.clone(), self.engine);
        let stitch_handler = StitchRecoveryHandler::new(self.engine);

        // try deleting error recovery
        let mut report = delete_recovery.handle(lookaheads.clone()).and_then(|candidate| {
            stitch_handler.try_recovery(candidate, lookaheads.clone().skip(self.penalty.delete_slot - delete_recovery.left_slot()))
        });

        // try shifting error recovery
        while let Some(candidate) = shift_recovery.handle(lookahead) {
            let next_report = stitch_handler.try_recovery(candidate, lookaheads.clone());
            match (report.as_ref(), next_report.as_ref()) {
                (None, Some(_)) => {
                    report = next_report;
                }
                (Some(lhs), Some(rhs)) if lhs.score < rhs.score => {
                    report = next_report;
                }
                _ => {}
            }
        }

        match report.as_ref() {
            Some(x) if x.recovery_method == Recovery::Delete => {
                self.penalty.accept_delete(x.score);
                Some(x.events())
            }
            Some(x) if x.recovery_method == Recovery::Shift => {
                self.penalty.accept_shift();
                Some(x.events())
            }
            _ => {
                None
            }
        }
    }
}

pub(crate) fn make_stack(state_histories: &[usize]) -> StateStack {
    let initial_state = state_histories.first().cloned().unwrap_or_default();

    let mut stack = StateStack::new(initial_state);

    for state in state_histories.iter().skip(1) {
        stack.push_state(*state);
    }

    stack
}

#[derive(Clone)]
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
    pub fn accept_delete(&mut self, used_slot: usize) {
        self.delete_slot -= used_slot;
    }
    pub fn accept_shift(&mut self) {
        self.shift_decay = self.next_shift_decay;
        self.next_shift_decay <<= 1;
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct RecoveryReport {
    recovery_method: Recovery,
    events: cactus::Cactus<(u32, RecoveryEvent)>,
    state_stack: StateStack,
    last_state: usize,
    score: usize,
    depth: usize,
}

impl RecoveryReport {
    pub fn new(failed_state: usize, state_histories: &[usize], recovery_method: Recovery) -> Self {
        Self::new_with_stack(failed_state, make_stack(state_histories), recovery_method)
    }

    pub(crate) fn new_with_stack(failed_state: usize, state_stack: StateStack, recovery_method: Recovery) -> Self {
        Self {
            recovery_method,
            events: cactus::Cactus::new(),
            state_stack,
            last_state: failed_state,
            score: 0,
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
    pub fn score(&self) -> usize {
        self.score
    }

    #[inline]
    pub fn reset_score(&mut self, new_score: usize) {
        self.score = new_score;
    }

    #[inline]
    pub fn method(&self) -> Recovery {
        self.recovery_method.clone()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum RecoveryEvent {
    /// A patch deleting action that was accepted during recovery.
    PatchDelete{ kind: SyntaxKind, state: usize },
    /// A patch shifting action that was accepted during recovery.
    PatchShift(RecoveryEventPayload),
    /// A normal parsing transition that occurred after patching
    Stitch(RecoveryEventPayload),
}

#[derive(PartialEq, Clone, Debug)]
pub enum RecoveryEventPayload {
    /// A Shift action performed during patch/stitch phase.
    Shift { kind: SyntaxKind, state: usize, next_state: usize },
    /// A Reduce action performed during patch/stitch phase.
    Reduce{ kind: SyntaxKind, state: usize, next_state: usize, pop_count: usize, },
    /// A Accept action performed during patch/stitch phase.
    Accept{ kind: SyntaxKind, state: usize, last_state: usize },
}