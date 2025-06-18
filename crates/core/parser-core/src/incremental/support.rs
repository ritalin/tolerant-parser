use std::collections::{HashMap, HashSet};

use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use rowan::NodeOrToken;
use crate::{metadata::{GlobalOffset, StatementMetadataEntry}, syntax_tree::{RowanLangageImpl, SyntaxNode}, NodeMetadata, NodeMetadataKey};

pub struct TreeGardener<'a> {
    pub node: rowan::SyntaxNode<RowanLangageImpl>,
    pub metadata_entry: &'a StatementMetadataEntry,
}

impl<'a> TreeGardener<'a> {
    pub fn new(stmt: &'a SyntaxNode) -> Self {
        Self {
            node: stmt.into_raw(),
            metadata_entry: stmt.metadata_entry()
        }
    }

    pub fn as_subtree(stmt: &'a SyntaxNode) -> Self {
        Self {
            node: stmt.into_raw().clone_subtree(),
            metadata_entry: stmt.metadata_entry()
        }
    }

    pub fn pick_token(&self, byte_offset: rowan::TextSize) -> Option<FoundToken> {
        match self.node.token_at_offset(byte_offset) {
            rowan::TokenAtOffset::None => return None,
            rowan::TokenAtOffset::Single(token) | rowan::TokenAtOffset::Between(_, token) => {
                Some(FoundToken{ token })
            }
        }
    }

    pub fn common_anscestor(&self, lhs: Option<FoundToken>, rhs: Option<FoundToken>, except_kind: SyntaxKind) -> Option<TreeGardener> {
        let (Some(lhs), Some(rhs)) = (lhs, rhs) else { return None; };
        
        // expand left hand token
        let left_neighbor = lhs.into_prev(&self.node, except_kind);
        // expand right hand token
        let right_beighbor = rhs.into_next(&self.node, except_kind);
        
        // Find least common anscestor
        let left_anscestors = left_neighbor.token.parent_ancestors().collect::<Vec<_>>();
        let right_anscestors = right_beighbor.token.parent_ancestors().collect::<Vec<_>>();
        let (lca, _) = left_anscestors.into_iter().rev().zip(right_anscestors.into_iter().rev())
            .take_while(|(lhs, rhs)| *lhs == *rhs)
            .last()
            .unzip()
        ;
        
        lca.and_then(|node| {
            let needle = node.text_range();
            node.ancestors().take_while(|anscestor| anscestor.text_range() == needle).last()
        })
        .map(|node| TreeGardener {
            node,
            metadata_entry: self.metadata_entry,
        })
    }

    pub fn pick_terminate_kind(&self, engine: ParsingRuleSet) -> IncrementalParserStrategy {
        let token = self.node.last_token().unwrap();
        
        let kind = match token.next_token() {
            Some(neighbor) => neighbor.parent().map(|x| engine.from_kind_id(x.kind())),
            None => None
        };

        let full_emit_kind = engine.full_emit_config().to_symbol;
        let terminate_kind = kind.unwrap_or(full_emit_kind);

        IncrementalParserStrategy{ full_emit_kind, terminate_kind }
    }

    pub fn replace_with_new_node(
        &self, 
        new_node: rowan::GreenNode,
        anscestor: &rowan::SyntaxNode<RowanLangageImpl>) -> rowan::GreenNode
    {
        let Some(parent) = anscestor.parent() else {
            return new_node;
        };
        let index = anscestor.index();

        let green_node = parent.green().splice_children(index..=index, vec![rowan::NodeOrToken::Node(new_node)]);
        parent.replace_with(green_node)
    }

    pub fn new_node_key(&self, node: &rowan::GreenNode, engine: ParsingRuleSet) -> NodeMetadataKey {
        NodeMetadataKey::from_green_node(node, 0, engine).into_global(self.node.text_range().start().into())
    }
}

#[derive(Clone)]
pub struct FoundToken {
    pub token: rowan::SyntaxToken<RowanLangageImpl>
}

impl FoundToken {
    pub fn into_prev(self, stmt: &rowan::SyntaxNode<RowanLangageImpl>, _except_kind: SyntaxKind) -> Self {
        let parent = self.token.parent().unwrap();
        let token = parent.first_token().unwrap();
        
        token.prev_token().map(|token| Self{ token })
        .filter(|x| x.is_ascendant(stmt))
        .unwrap_or(self)
    }

    pub fn into_next(self, stmt: &rowan::SyntaxNode<RowanLangageImpl>, except_kind: SyntaxKind) -> Self {
        if self.token.kind() == except_kind.id { return self; };

        let parent = self.token.parent().unwrap();
        let token = parent.last_token().unwrap();
        
        token.next_token().map(|token| Self{ token })
        .filter(|x| x.is_ascendant(stmt))
        .unwrap_or(self)
    }

    pub fn is_ascendant(
        &self,
        stmt: &rowan::SyntaxNode<RowanLangageImpl>) -> bool 
    {
        self.token.parent_ancestors().any(|x| x == *stmt)
    }
}

pub struct IncrementalParserStrategy {
    full_emit_kind: SyntaxKind,
    terminate_kind: SyntaxKind,
}

impl IncrementalParserStrategy {
    pub fn default_strategy(engine: ParsingRuleSet) -> Self {
        let kind = engine.full_emit_config().to_symbol;

        Self {
            full_emit_kind: kind,
            terminate_kind: kind,
        }
    }
}

impl crate::parser::ParseStrategy for IncrementalParserStrategy {
    fn is_terminated_kind(&self, kind: SyntaxKind, scanner: &impl scanner_core::ScannerAccess) -> bool {
        if self.terminate_kind != self.full_emit_kind {
            if let Some(token) = scanner.lookahead() {
                return token.main.kind == self.full_emit_kind;
            }
        }
        self.terminate_kind == kind
    }
}

pub(crate)trait  IncludeEnd {
    type Item;
    fn include_end(self) -> std::ops::RangeInclusive<Self::Item>;
}

impl<T> IncludeEnd for std::ops::Range<T> {
    type Item = T;

    fn include_end(self) -> std::ops::RangeInclusive<Self::Item> {
        self.start..=self.end
    }
}

pub fn adjust_edit_range(base_range: &std::ops::Range<usize>, node_byte_range: &std::ops::Range<usize>) -> std::ops::Range<u32> {
    let lowest_offset = u32::max(base_range.start as u32, node_byte_range.start as u32);
    let highest_offset = u32::min(base_range.end as u32, node_byte_range.end as u32);
    
    lowest_offset..highest_offset
}

pub fn merge_metadata_map(
    old_pair: Option<(rowan::SyntaxNode<RowanLangageImpl>, &HashMap<NodeMetadataKey, NodeMetadata>)>,
    (new_anscestor, new_metadata): (&rowan::GreenNode, HashMap<NodeMetadataKey, NodeMetadata>),
    global_byte_offset: usize, local_char_offset: usize,
    engine: ParsingRuleSet) -> StatementMetadataEntry
{
    let mut new_metadata_map = HashMap::from_iter(
        new_metadata.into_iter()
        .map(|(key, metadata)| {
            (key.into_local(global_byte_offset), metadata.into_global(local_char_offset))
        })
    );

    if let Some((old_anscestor, old_metadata)) = old_pair {
        let anscestor_range: std::ops::Range<usize> = old_anscestor.text_range().into();
        let old_char_len = measure_char_len(old_anscestor.green().as_ref());
        let new_char_len = measure_char_len(std::borrow::Borrow::borrow(new_anscestor));

        let old_byte_len: usize = anscestor_range.len();
        let new_byte_len: usize = new_anscestor.text_len().into();

        let anscestor_path = old_anscestor.ancestors()
            .map(|x| NodeMetadataKey::from_raw_node(&x, engine))
            .collect::<HashSet<_>>()
        ;

        // Phase1: merge metadata except anscestors
        old_metadata.iter()
            .filter(|(key, _)| {
                !anscestor_path.contains(key)
            })
            .filter_map(|(key, metadata)| match (key.offset, key.len) {
                (offset, len) if offset + len <= anscestor_range.start => {
                    // Before anscestor nodes descendants
                    Some((key.clone(), metadata.clone()))
                }
                (offset, _) if offset >= anscestor_range.end => {
                    // After anscestor node descendants
                    let key = NodeMetadataKey{ offset: key.offset + new_byte_len - old_byte_len, ..key.clone() };
                    let metadata = NodeMetadata { char_offset: metadata.char_offset + new_char_len - old_char_len, ..metadata.clone() };
                    Some((key, metadata))
                }
                _ => {
                    // Ignore anscestor node descendants
                    None
                }
            })
            .for_each(|(key, metadata)| {
                new_metadata_map.insert(key, metadata);
            })
        ;

        // Phase2: regenerate anscestors metadata
        for node in old_anscestor.ancestors() {                
            let mut key = NodeMetadataKey::from_raw_node(&node, engine);
            let mut metadata = old_metadata.get(&key).expect("All of nodes need to have a metadata").clone();

            key.len = key.len + new_byte_len - old_byte_len;
            metadata.char_len = metadata.char_len + new_char_len - old_char_len;

            new_metadata_map.insert(key, metadata);
        }
    }

    // each offsets is updated latter
    return StatementMetadataEntry {
        global_offset: GlobalOffset::default(),
        map: new_metadata_map,
    };
}

fn measure_char_len(node: &rowan::GreenNodeData) -> usize {
    let mut acc = 0; 
    measure_char_len_internal(rowan::NodeOrToken::Node(node), &mut acc);

    acc
}

fn measure_char_len_internal(node: NodeOrToken<&rowan::GreenNodeData, &rowan::GreenTokenData>, acc: &mut usize) {
    let mut stack = vec![node];

    while let Some(el) = stack.pop() {
        match el {
            NodeOrToken::Node(node) => {
                stack.extend(node.children());
            }
            NodeOrToken::Token(token) => {
                *acc += token.text().chars().count();
            }
        };
    }
}
