use std::collections::VecDeque;

use engine_core::{parser_engine::{ParsingRuleSet, Transition}, SymbolGroup, SyntaxKind};
use scanner_core::Token;
use crate::{state_stack::StateStack, Recovery};

use super::{RecoveryEvent, RecoveryEventPayload, RecoveryPenalty, RecoveryReport};

pub struct ShiftErrorRecovery {
    candidates: VecDeque<RecoveryReport>,
    wait_list: Vec<Packet>,
    penalty: RecoveryPenalty,
    engine: ParsingRuleSet,
}

impl ShiftErrorRecovery {
    pub fn new(failed_state: usize, state_histories: &[usize], penalty: RecoveryPenalty, engine: ParsingRuleSet) -> Self {
        Self::new_with_stack(failed_state, super::make_stack(state_histories), penalty, engine)
    }

    pub(crate) fn new_with_stack(failed_state: usize, state_stack: StateStack, penalty: RecoveryPenalty, engine: ParsingRuleSet) -> Self {
        let report = RecoveryReport::new_with_stack(failed_state, state_stack, Recovery::Shift);

        Self {
            candidates: VecDeque::with_capacity(0),
            wait_list: next_candidates_internal(report, None, engine).collect(),
            penalty, 
            engine,
        }
    }

    pub fn handle(&mut self, lookahead: &Token) -> Option<RecoveryReport> {
        loop {
            if let Some(report) = self.candidates.pop_front() {
                return Some(report);
            }

            // Fill candidates
            let mut wait_list = Vec::with_capacity(0);
            std::mem::swap(&mut self.wait_list, &mut wait_list);
            (self.candidates, self.wait_list) = next_candidates(wait_list.into_iter(), lookahead, &self.penalty, self.engine);

            if self.wait_list.is_empty() && self.candidates.is_empty() {
                // No more candidate
                return None;
            }
        }
    }
}

#[derive(Clone)]
struct Packet {
    kind_id: u32,
    report: RecoveryReport,
}

// action group = shift, reduce, reduce_opt
const N_ACTION: usize = 3;
// symbol group = keyword, non-keyword, regex-pattern
const N_SYMBOL: usize = 3;

fn next_candidates(prev_candidates: impl Iterator<Item = Packet>, lookahead: &Token, penalty: &RecoveryPenalty, engine: ParsingRuleSet) -> (VecDeque<RecoveryReport>, Vec<Packet>) {
    let limit = penalty.max_shift_packet_size * N_ACTION * N_SYMBOL;
    let mut next_candidate = VecDeque::with_capacity(prev_candidates.size_hint().0);
    let mut next_wait_list = Vec::with_capacity(limit * 2);

    for prev in prev_candidates {
        if prev.kind_id == lookahead.main.kind.id {
            // Candidate found
            next_candidate.push_back(prev.report);
            continue;
        }
        if next_wait_list.len() >= limit { continue }
        if prev.report.contains_kind(lookahead.main.kind) { continue }

        if prev.report.depth < penalty.shift_limit - penalty.shift_decay {
            // make next wait list
            next_wait_list.extend(next_candidates_internal(prev.report, Some(lookahead.main.kind), engine));
        }
    }

    (next_candidate, next_wait_list)
}

fn next_candidates_internal(report: RecoveryReport, lookahead_kind: Option<SyntaxKind>, engine: ParsingRuleSet) -> impl Iterator<Item = Packet> {
    let mut packets: [Option<Packet>; N_ACTION * (N_SYMBOL + 1)] = std::array::from_fn(|_| None);

    for symbol in engine.candidate_terminal_symbols(report.last_state) {
        let col = lookahead_kind
            .and_then(|k| (symbol.id == k.id).then(|| 0))
            .or_else(|| find_packet_group_column(symbol))
        ;
        let Some(col) = col else { continue };

        let kind = symbol.clone();

        match engine.next_lookahead_state(symbol.id, report.last_state) {
            Some(Transition::Shift { next_state }) if packets[col].is_none() => {
                let mut next_report = report.next_report();
                
                next_report.state_stack.push_state(*next_state);
                next_report.push_event(kind.id, 
                    RecoveryEvent::PatchShift(RecoveryEventPayload::Shift { 
                        kind,
                        state: report.last_state, 
                        next_state: *next_state 
                    })
                );
                next_report.last_state = *next_state;
                next_report.score += 1;

                packets[col] = Some(Packet{ kind_id: symbol.id, report: next_report });
            }
            Some(Transition::Reduce { pop_count, lhs }) if (*pop_count > 0) && packets[col + N_SYMBOL].is_none() => {
                let mut next_report = report.next_report();

                let Some(goto_state) = next_report.state_stack.pop_n_state(*pop_count) else { continue };
                let Some(next_state) = engine.next_goto_state(*lhs, *goto_state) else { continue };

                next_report.state_stack.push_state(*next_state);
                next_report.push_event(kind.id, 
                    RecoveryEvent::PatchShift(super::RecoveryEventPayload::Reduce { 
                        kind: engine.from_kind_id(*lhs),
                        state: report.last_state, 
                        next_state: *next_state, 
                        pop_count: *pop_count 
                    })
                );
                next_report.last_state = *next_state;
                next_report.score += 1;
 
                packets[col + N_SYMBOL] = Some(Packet{ kind_id: symbol.id, report: report.next_report() })
            }
            Some(Transition::Reduce { pop_count, lhs }) if packets[col + N_SYMBOL * 2].is_none() => {
                let mut next_report = report.next_report();

                let Some(goto_state) = next_report.state_stack.pop_n_state(*pop_count) else { continue };
                let Some(next_state) = engine.next_goto_state(*lhs, *goto_state) else { continue };

                next_report.state_stack.push_state(*next_state);
                next_report.push_event(kind.id, 
                    RecoveryEvent::PatchShift(super::RecoveryEventPayload::Reduce { 
                        kind: engine.from_kind_id(*lhs), 
                        state: report.last_state, 
                        next_state:  *next_state, 
                        pop_count: *pop_count
                    })
                );
                next_report.last_state = *next_state;

                packets[col + N_SYMBOL * 2] = Some(Packet{ kind_id: symbol.id, report: report.next_report() })
            }
            _ => {
            }
        }
    }

    packets.into_iter().flatten()
}

fn find_packet_group_column(symbol: &SyntaxKind) -> Option<usize> {
    match symbol.group {
        SymbolGroup::Pattern => Some(1),
        SymbolGroup::NonKeyword => Some(2),
        SymbolGroup::Keyword => Some(3),
        SymbolGroup::NonTerminal => None,
    }
}