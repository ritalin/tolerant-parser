use std::collections::VecDeque;
use parser_core::{syntax_tree::{FragmentNode, FragmentNodeMetadataKey, MetadataAccess, SyntaxElement, SyntaxFragmentBatch, SyntaxNode, SyntaxTree}, GlobalOffset, NodeMetadataKey};

#[derive(PartialEq, Debug)]
pub enum PrintBatch {
    Node(FragmentNode),
    BeginBatch(SlotSizeHint),
    EndBatch,
    Source { text: String, line_size: isize },
}

#[derive(PartialEq, Clone, Debug)]
pub struct SlotSizeHint {
    pub offset: usize,
    pub old_size: usize,
    pub new_size: usize,
}

pub struct PrintBatchPipeline {
    batches: Vec<SyntaxFragmentBatch>,
    batch_index: usize,
    fragment_index: usize,
    fragment_offset_hints: Vec<(SlotSizeHint, Vec<GlobalOffset>)>,
    pipleine_cache: VecDeque<PrintBatch>
}

impl PrintBatchPipeline {
    pub fn new(old_tree: SyntaxTree, batches: Vec<SyntaxFragmentBatch>) -> Self {
        let fragment_offset_hints = cache_fragment_offset(Some(&old_tree), &batches);
        let pipleine_cache = match fragment_offset_hints.first() {
            Some((first_hint, _)) => VecDeque::from([PrintBatch::BeginBatch(first_hint.clone())]),
            None => VecDeque::new(),
        };

        Self {
            batches,
            batch_index: 0,
            fragment_index: 0,
            fragment_offset_hints,
            pipleine_cache,
        }
    }

    fn fill_cache(&mut self) {
        while let Some(batch) = self.batches.get(self.batch_index) {
            let (_, fragment_offsets) = &self.fragment_offset_hints[self.batch_index];

            while let Some(fragment) = batch.fragments.get(self.fragment_index) {
                // FIXME: Send source

                let offset = fragment_offsets[self.fragment_index].clone();
                self.pipleine_cache.extend(fragment.iter(offset, batch.engine).map(PrintBatch::Node));
                self.fragment_index += 1;
                return;
            }

            self.pipleine_cache.push_back(PrintBatch::EndBatch);

            self.batch_index += 1;
            self.fragment_index = 0;

            if self.batch_index < self.batches.len() {
                let (slot_hint, _) = &self.fragment_offset_hints[self.batch_index];
                self.pipleine_cache.push_back(PrintBatch::BeginBatch(slot_hint.clone()));
            }
        }
    }

    pub fn batch_size(&self) -> usize {
        self.batches.len()
    }
}

impl Iterator for PrintBatchPipeline {
    type Item = PrintBatch;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pipleine_cache.is_empty() {
            self.fill_cache();
        }
        
        self.pipleine_cache.pop_front()
    }
}

impl From<&SyntaxTree> for PrintBatchPipeline {
    fn from(value: &SyntaxTree) -> PrintBatchPipeline {
        todo!()
    }
}

fn cache_fragment_offset(old_tree: Option<&SyntaxTree>, batches: &[SyntaxFragmentBatch]) -> Vec<(SlotSizeHint, Vec<GlobalOffset>)> {
    let mut old_global_offsets = vec![GlobalOffset::default(); batches.len()];
    let mut old_node_slots = vec![0..0; batches.len()]; // slot start .. slot end

    if let Some(tree) = old_tree {
        enumerate_old_global_offsets(&batches, &tree.root(), &mut old_global_offsets);
        enumerate_old_slot_spans(&batches, &tree.root(), &mut old_node_slots);
    }

    let mut all_cache = vec![];
    let mut last_cache = GlobalOffset::default();

    for ((batch, offset), slot) in batches.iter().zip(old_global_offsets).zip(old_node_slots) {
        last_cache.of_byte += offset.of_byte;
        last_cache.of_char += offset.of_char;

        let fragment_offsets = batch.fragments.iter()
            .map(|fragment| {
                let cache = last_cache.clone();
                last_cache.of_byte += fragment.statement_byte_len();
                last_cache.of_char += fragment.statement_char_len();

                cache
            })
            .collect::<Vec<_>>()
        ;
        let new_statement_node_count = batch.statement_node_count();

        let slot_hint = SlotSizeHint { 
            offset: slot.start, 
            old_size: slot.len(), 
            new_size: if batch.replace_size > 0 { new_statement_node_count - slot.start } else { new_statement_node_count }
        };
        all_cache.push((slot_hint, fragment_offsets));
    }

    all_cache
}

fn enumerate_old_global_offsets(batches: &[SyntaxFragmentBatch], root_node: &SyntaxNode, offsets: &mut Vec<GlobalOffset>) {
    let offset_bitmap = batches.iter()
        .enumerate()
        .scan(0, |last_index, (i, batch)| {
            let mapped = (*last_index..(batch.replace_from)).map(move |_| Some(i));
            let skipped = ((batch.replace_from)..(batch.replace_from + batch.replace_size)).map(|_| None);
            Some(mapped.chain(skipped))
        })
        .flatten()
    ;
    for (index, child) in root_node.children().zip(offset_bitmap).filter_map(|(child, index)| index.zip(Some(child))) {
        offsets[index].of_byte += child.metadata_key().len;
        offsets[index].of_char += child.metadata().char_len;
    }}

fn enumerate_old_slot_spans(batches: &[SyntaxFragmentBatch], root_node: &SyntaxNode, spans: &mut Vec<std::ops::Range<usize>>) {
    for (index, batch) in batches.iter().enumerate() {
        match batch.old_first_fragment_key.as_ref() {
            Some(FragmentNodeMetadataKey{ key: old_key, is_eof }) => {
                // determin slot start
                if let Some((slot_start, sub_node)) = find_slot_start(root_node, old_key) {
                    let slot_size = match is_eof {
                        true => 0,
                        false => {
                            // determin first slot end
                            let first_node_count = count_descendant_nodes(&sub_node);
                            // determin slot end from following statement count
                            root_node.children().skip(batch.replace_from + 1).take(batch.replace_size.saturating_sub(1))
                            .fold(first_node_count, |acc, stmt| {
                                let count = count_descendant_nodes(&stmt);
                                acc + count
                            })
                        }
                    };

                    spans[index] = slot_start..(slot_start + slot_size);
                }
            }
            None => {

            }
        }
    }
}

fn find_slot_start(root_node: &SyntaxNode, needle: &NodeMetadataKey) -> Option<(usize, SyntaxElement)> {
    let mut index = 0;

    for el in root_node.descendant_nodes() {
        match &el {
            parser_core::syntax_tree::SyntaxElementDef::Node(node) if node.metadata_key() == *needle => {
                return Some((index, el));
            }
            parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) if token_set.metadata_key() == *needle => {
                return Some((index, el));
            }
            parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) => {
                index += token_set.descendant_tokens().count();
            }
            _ => {}
        }

        index += 1;
    }

    None
}

fn count_descendant_nodes(el: &SyntaxElement) -> usize {
    match el {
        SyntaxElement::Node(node) => {
            // enumerate descendants except for root_node
            node.descendant_nodes().skip(1).map(|node| match node {
                parser_core::syntax_tree::SyntaxElementDef::Node(_) => {
                    1
                }
                parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) => {
                    token_set.descendant_tokens().count() + 1
                }
            })
            .sum()
        }
        SyntaxElement::TokenSet(token_set) => {
            token_set.descendant_tokens().count()
        }
    }
}
