use std::collections::HashMap;
use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use crate::{metadata::{MetadataTable, StatementMetadataEntry}, syntax_tree::{tree::SyntaxTree, RowanLangageImpl, SyntaxNode, SyntaxNodeData}, NodeMetadata, NodeMetadataKey, ParseMode};

pub struct SyntaxFragment {
    index: usize,
    node: rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>,
    metadata_entry: StatementMetadataEntry,
    adjusted_byte_offset: usize,
}

impl SyntaxFragment {
    pub fn new(index: usize, node: rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>, metadata_entry: StatementMetadataEntry) -> Self {
        Self {
            index,
            node,
            metadata_entry,
            adjusted_byte_offset: 0,
        }
    }

    pub fn adjust_byte_offset(self, byte_offset: usize) -> Self {
        Self {
            adjusted_byte_offset: byte_offset,
            ..self
        }
    }

    pub fn into_root(self, parse_mode: ParseMode, engine: ParsingRuleSet) -> SyntaxNode {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let metadata_table = MetadataTable::new(vec![self.metadata_entry], StatementMetadataEntry::default());

        SyntaxNode::from_raw(SyntaxNodeData::new(red_node, std::rc::Rc::new(metadata_table), parse_mode, engine))
    }

    #[inline]
    pub fn seq(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.adjusted_byte_offset
    }
}

impl std::fmt::Display for SyntaxFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.node.clone().into_node().unwrap());

        let mut next_token = red_node.first_token();
        while let Some(token) = next_token {
            write!(f, "{}", token.text())?;
            next_token = token.next_token();
        }

        Ok(())
    }
}

pub struct SyntaxFragmentBatch {
    pub fragments: Vec<SyntaxFragment>,
    pub replace_from: usize,
    pub replace_size: usize,
    pub engine: ParsingRuleSet,
}

impl From<SyntaxFragmentBatch> for SyntaxTree {
    fn from(value: SyntaxFragmentBatch) -> Self {
        let mut members = value.fragments;
        members.sort_by(|lhs, rhs| lhs.seq().cmp(&rhs.seq()));

        let mut global_byte_offset = 0;
        let mut global_char_offset = 0;

        let (children, metadata_entriess): (Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, Vec<StatementMetadataEntry>) = 
            members.into_iter()
            .map(|member| {
                let metadata_entry = StatementMetadataEntry{ 
                    byte_offset: global_byte_offset, 
                    char_offset: global_char_offset, 
                    map: member.metadata_entry.map 
                };
                global_byte_offset += usize::from(member.node.text_len());
                global_char_offset += measure_statement_char_len(std::borrow::Borrow::borrow(member.node.as_node().unwrap()));

                (member.node, metadata_entry)
            })
            .unzip()
        ;

        let root_metadata_entry = new_root_metadata(value.engine.full_emit_config().from_symbol, global_byte_offset, global_char_offset);
        let root_kind_id = rowan::SyntaxKind(value.engine.full_emit_config().from_symbol.id as u16);
        let root_node = rowan::GreenNode::new(root_kind_id, children);

        let metadata_table = MetadataTable::new(metadata_entriess, root_metadata_entry);

        SyntaxTree::new(root_node, metadata_table, ParseMode::ByStatement, value.engine)
    }
}

fn measure_statement_char_len(root: &rowan::GreenNodeData) -> usize {
    let mut stack = vec![rowan::NodeOrToken::Node(root)];
    let mut size = 0;

    while let Some(node) = stack.pop() {
        match node {
            rowan::NodeOrToken::Node(node) => {
                stack.extend(node.children());
            }
            rowan::NodeOrToken::Token(token) => {
                size += token.text().chars().count();
            }
        }
    }

    size
}

fn new_root_metadata(kind: SyntaxKind, byte_offset: usize, char_offset: usize) -> StatementMetadataEntry {
    let root_key = NodeMetadataKey{ 
        kind, 
        offset: 0, len: byte_offset, is_leaf: false 
    };
    let root_metadata = NodeMetadata{ 
        edit_state: 0, 
        node_type: crate::NodeType::Node, 
        patch: crate::PatchAction::None, 
        char_offset: 0, 
        char_len: char_offset 
    };
    
    StatementMetadataEntry{ 
        map: HashMap::from([(root_key, root_metadata)]), 
        ..Default::default() 
    }
}

pub trait ApplyBatch {
    type Output;
    fn apply_batches(&self, batches: Vec<SyntaxFragmentBatch>) -> Self::Output;
}

pub fn apply_batches<Batch>(
    root: &rowan::SyntaxNode<RowanLangageImpl>, 
    metadata_table: &MetadataTable, 
    engine: ParsingRuleSet, 
    batches: Batch) -> SyntaxTree
where Batch: IntoIterator<Item = SyntaxFragmentBatch>
{
    // FIXME: apply all batches
    let Some(batch) = batches.into_iter().next() else { panic!("At least SyntaxFragmentBatch is needed") };
    let batch_range = batch.replace_from..(batch.replace_from + batch.replace_size);

    let (children, metadata_entries): (Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, Vec<StatementMetadataEntry>) = 
        batch.fragments.into_iter()
        .map(|SyntaxFragment { node, metadata_entry, .. }| (node, metadata_entry))
        .unzip()
    ;
    let mut new_metadata_entries = metadata_table.clone_members();
    new_metadata_entries.splice(batch_range.clone(), metadata_entries);

    let new_root = root.green().splice_children(batch_range, children);

    // Update statement offset
    let mut global_byte_offset = 0;
    let mut global_char_offset = 0;

    new_metadata_entries.iter_mut().zip(new_root.children())
    .for_each(|(entry, node)| {
        entry.byte_offset = global_byte_offset;
        entry.char_offset = global_char_offset;

        global_byte_offset += usize::from(node.text_len());
        global_char_offset += measure_statement_char_len(std::borrow::Borrow::borrow(node.as_node().unwrap()));
    });

    let root_metadata_entry = new_root_metadata(engine.full_emit_config().from_symbol, global_byte_offset, global_char_offset);
    let new_metadata_table = MetadataTable::new(new_metadata_entries, root_metadata_entry);

    SyntaxTree::new(new_root, new_metadata_table, ParseMode::ByStatement, engine)
}