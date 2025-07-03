use std::collections::{HashSet, VecDeque};

use engine_core::{parser_engine::{ParsingRuleSet, Transition}, SymbolGroup, SyntaxKind};
use scanner_core::Token;
use crate::state_stack::StateStack;

use super::{RecoveryEvent, RecoveryEventPayload, RecoveryPenalty, RecoveryReport};

pub struct ShiftErrorRecovery {
    candidates: VecDeque<RecoveryReport>,
    wait_list: Vec<Packet>,
    penalty: RecoveryPenalty,
    except_kind_ids: HashSet<u32>,
    engine: ParsingRuleSet,
}

impl ShiftErrorRecovery {
    #[cfg(feature = "test_support")]
    #[doc(hidden)]
    pub fn new(state_histories: &[usize], penalty: RecoveryPenalty, except_kinds: &HashSet<u32>, engine: ParsingRuleSet) -> Self {
        Self::new_with_stack(super::make_stack(state_histories), penalty, except_kinds, engine)
    }

    pub(crate) fn new_with_stack(state_stack: StateStack, penalty: RecoveryPenalty, except_kind_ids: &HashSet<u32>, engine: ParsingRuleSet) -> Self {
        let report = RecoveryReport::new_with_stack(state_stack);

        Self {
            candidates: VecDeque::with_capacity(0),
            wait_list: next_candidates_internal(report, None, None, except_kind_ids, engine).collect(),
            penalty, 
            except_kind_ids: except_kind_ids.clone(),
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
            (self.candidates, self.wait_list) = next_candidates(wait_list.into_iter(), lookahead, &self.except_kind_ids, &self.penalty, self.engine);

            if self.wait_list.is_empty() && self.candidates.is_empty() {
                // No more candidate
                return None;
            }
        }
    }
}

#[derive(Clone)]
struct Packet {
    // Picked up the lookahead kind
    kind_id: u32,
    is_reduce: bool,
    report: RecoveryReport,
}

// action group = shift, reduce, reduce_opt
const N_ACTION: usize = 3;
// symbol group = keyword, non-keyword, regex-pattern
const N_SYMBOL: usize = 3;

fn next_candidates(prev_candidates: impl Iterator<Item = Packet>, lookahead: &Token, except_kind_ids: &HashSet<u32>, penalty: &RecoveryPenalty, engine: ParsingRuleSet) -> (VecDeque<RecoveryReport>, Vec<Packet>) {
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

        if prev.report.depth + penalty.shift_decay < penalty.shift_limit {
            // make next wait list

            let prev_kind = match prev.is_reduce {
                true => {
                    // If preceding action is Recuce, the parser must use the same lookahead
                    Some(engine.from_kind_id(prev.kind_id))
                }
                false => None,
            };

            next_wait_list.extend(next_candidates_internal(
                prev.report, prev_kind.as_ref(), 
                Some(lookahead.main.kind), except_kind_ids, engine));
        }
    }

    (next_candidate, next_wait_list)
}

fn next_candidates_internal(report: RecoveryReport, prev_kind: Option<&SyntaxKind>, lookahead_kind: Option<SyntaxKind>, except_kind_ids: &HashSet<u32>, engine: ParsingRuleSet) -> impl Iterator<Item = Packet> {
    let mut packets: [Option<Packet>; N_ACTION * (N_SYMBOL + 1)] = std::array::from_fn(|_| None);

    if let Some(last_state) = report.state_stack.peek_state().cloned() {
        let candidates = match prev_kind {
            None => engine.candidate_terminal_symbols(last_state),
            Some(kind) => vec![kind],
        };
    
        for symbol in candidates {
            if report.contains_kind(*symbol) { continue }

            match lookahead_kind {
                Some(kind) if symbol.id == kind.id => {
                    // Candidate found
                    packets[0] = Some(Packet{ kind_id: symbol.id, is_reduce: false, report: report.clone() });
                    continue;
                }
                _ => {}
            }

            if except_kind_ids.contains(&symbol.id) { 
                // Skip shift symbol
                continue 
            }

            let Some(col) = find_packet_group_column(symbol) else { continue };

            let kind = symbol.clone();

            match engine.next_lookahead_state(symbol.id, last_state) {
                Some(Transition::Shift { next_state }) if packets[col].is_none() => {
                    let mut next_report = report.next_report();
                    
                    next_report.state_stack.push_state(*next_state);
                    next_report.push_event(kind.id, 
                        RecoveryEvent::PatchShift(RecoveryEventPayload::Shift { 
                            kind,
                            state: last_state, 
                            next_state: *next_state 
                        })
                    );
                    next_report.patch_score += 1;

                    packets[col] = Some(Packet{ kind_id: symbol.id, is_reduce: false, report: next_report });
                }
                Some(Transition::Reduce { pop_count, lhs }) if (*pop_count > 0) && packets[col + N_SYMBOL].is_none() => {
                    if except_kind_ids.contains(lhs) { 
                        // Skip reduce symbol
                        continue 
                    }
                    let mut next_report = report.next_report();

                    let Some(goto_state) = next_report.state_stack.pop_n_state(*pop_count) else { continue };
                    let Some(next_state) = engine.next_goto_state(*lhs, *goto_state) else { continue };

                    let lhs_kind = engine.from_kind_id(*lhs);
                    next_report.state_stack.push_state(*next_state);
                    next_report.push_event(lhs_kind.id, 
                        RecoveryEvent::PatchShift(super::RecoveryEventPayload::Reduce { 
                            kind: lhs_kind,
                            state: last_state, 
                            next_state: *next_state, 
                            pop_count: *pop_count 
                        })
                    );
                    next_report.patch_score += 1;
    
                    packets[col + N_SYMBOL] = Some(Packet{ kind_id: symbol.id, is_reduce: true, report: next_report })
                }
                Some(Transition::Reduce { pop_count, lhs }) if packets[col + N_SYMBOL * 2].is_none() => {
                    if except_kind_ids.contains(lhs) { 
                        // Skip reduce symbol
                        continue 
                    }
                    let mut next_report = report.next_report();

                    let Some(goto_state) = next_report.state_stack.pop_n_state(*pop_count) else { continue };
                    let Some(next_state) = engine.next_goto_state(*lhs, *goto_state) else { continue };

                    let lhs_kind = engine.from_kind_id(*lhs);
                    next_report.state_stack.push_state(*next_state);
                    next_report.push_event(lhs_kind.id, 
                        RecoveryEvent::PatchShift(super::RecoveryEventPayload::Reduce { 
                            kind: lhs_kind, 
                            state: last_state, 
                            next_state:  *next_state, 
                            pop_count: *pop_count
                        })
                    );

                    packets[col + N_SYMBOL * 2] = Some(Packet{ kind_id: symbol.id, is_reduce: true, report: next_report })
                }
                _ => {
                }
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