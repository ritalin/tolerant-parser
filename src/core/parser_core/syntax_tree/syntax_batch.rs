use std::collections::HashMap;
use crate::core::engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use crate::core::parser_core::{self, metadata::{GlobalOffset, MetadataTable, StatementMetadataEntry}, syntax_tree::{tree::SyntaxTree, MetadataAccess, RowanLangageImpl, SyntaxNode, SyntaxNodeData}, NodeMetadata, NodeMetadataKey, ParseMode};

pub struct SyntaxFragment {
    index: usize,
    new_statement: rowan::GreenNode,
    fragment_node_key: NodeMetadataKey,
    metadata_entry: StatementMetadataEntry,
    adjusted_byte_offset: usize,
}

impl SyntaxFragment {
    pub fn new(index: usize, new_statement: rowan::GreenNode, metadata_entry: StatementMetadataEntry, node_key: NodeMetadataKey) -> Self {
        Self {
            index,
            new_statement,
            fragment_node_key: node_key,
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
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.new_statement.clone());

        let metadata_table = MetadataTable::new(vec![self.metadata_entry], StatementMetadataEntry::default());

        SyntaxNode::from_raw(SyntaxNodeData::new(red_node, std::rc::Rc::new(metadata_table), parse_mode, engine))
    }

    pub fn iter(&self, global_offset: GlobalOffset, engine: ParsingRuleSet) -> FragmentNodeIterator {
        FragmentNodeIterator::new(&self.new_statement, &self.metadata_entry, self.fragment_node_key.clone(), global_offset, engine)
    }

    #[inline]
    pub fn seq(&self) -> usize {
        self.index
    }

    #[inline]
    pub fn byte_offset(&self) -> usize {
        self.adjusted_byte_offset
    }

    pub fn statement_byte_len(&self) -> usize {
        self.new_statement.text_len().into()
    }

    pub fn statement_char_len(&self) -> usize {
        measure_statement_char_len(&self.new_statement)
    }
}

impl std::fmt::Display for SyntaxFragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let red_node = rowan::api::SyntaxNode::<RowanLangageImpl>::new_root(self.new_statement.clone());

        let mut next_token = red_node.first_token();
        while let Some(token) = next_token {
            write!(f, "{}", token.text())?;
            next_token = token.next_token();
        }

        Ok(())
    }
}

pub struct FragmentNodeIterator<'a> {
    depth: usize,
    preorder: rowan::api::PreorderWithTokens<RowanLangageImpl>,
    metadata_entry: &'a StatementMetadataEntry,
    global_offset: GlobalOffset,
    engine: ParsingRuleSet,
}

impl<'a> FragmentNodeIterator<'a> {
    pub(crate) fn new(
        node: &rowan::GreenNode,
        metadata_entry: &'a StatementMetadataEntry,
        fragment_node_key: NodeMetadataKey,
        global_offset: GlobalOffset,
        engine: ParsingRuleSet) -> Self 
    {
        let fragment_node = find_fragment_node(node, fragment_node_key, engine);
        let fragment_depth = fragment_node.ancestors().count();

        Self {
            depth: fragment_depth,
            preorder: fragment_node.preorder_with_tokens(),
            metadata_entry: metadata_entry,
            global_offset,
            engine,
        }
    }
}

fn find_fragment_node(root_node: &rowan::GreenNode, needle: NodeMetadataKey, engine: ParsingRuleSet) -> rowan::SyntaxNode::<RowanLangageImpl> {
    let node = rowan::SyntaxNode::<RowanLangageImpl>::new_root(root_node.clone());
    let token = match node.token_at_offset(rowan::TextSize::new(needle.offset as u32)) {
        rowan::TokenAtOffset::None => {
            return node;
        }
        rowan::TokenAtOffset::Single(token) => token,
        rowan::TokenAtOffset::Between(_, token) => token,
    };

    token.parent_ancestors().find(|node| {
        let key = NodeMetadataKey::from_raw_node(node, engine);
        key == needle
    })
    .unwrap_or(node)
}

impl<'a> Iterator for FragmentNodeIterator<'a> {
    type Item = FragmentNode;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(node) = self.preorder.next() {
            match node {
                rowan::WalkEvent::Enter(node) => {
                    self.depth += 1;
                    let key = NodeMetadataKey{ 
                        kind: self.engine.from_kind_id(node.kind() as u32), 
                        offset: node.text_range().start().into(), 
                        len: node.text_range().len().into(), 
                        is_leaf: node.as_token().is_some() 
                    };
                    let metadata = self.metadata_entry.map
                        .get(&key).expect("A metadata definitely must exist").clone()
                    ;
                    return Some(FragmentNode{ 
                        node, 
                        key: key.into_global(self.global_offset.of_byte), 
                        metadata: metadata.into_global(self.global_offset.of_char), 
                        depth: self.depth - 1
                    });
                }
                rowan::WalkEvent::Leave(_) => {
                    self.depth -= 1;
                }
            }
        }
        
        None
    }
}

#[derive(PartialEq, Debug)]
pub struct FragmentNode {
    node: rowan::NodeOrToken<rowan::SyntaxNode<RowanLangageImpl>, rowan::SyntaxToken<RowanLangageImpl>>,
    key: NodeMetadataKey,
    metadata: NodeMetadata,
    depth: usize,
}

impl FragmentNode {
    pub fn value(&self) -> Option<String> {
        match &self.node {
            rowan::NodeOrToken::Node(_) => None,
            rowan::NodeOrToken::Token(token) => Some(token.text().into()),
        }
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

impl MetadataAccess for FragmentNode {
    fn metadata_key(&self) -> NodeMetadataKey {
        self.key.clone()
    }

    fn metadata(&self) -> NodeMetadata {
        self.metadata.clone()
    }
}

pub struct FragmentNodeMetadataKey {
    pub key: NodeMetadataKey,
    pub is_eof: bool,
}

pub struct SyntaxFragmentBatch {
    pub fragments: Vec<SyntaxFragment>,
    pub replace_from: usize,
    pub replace_size: usize,
    pub old_first_fragment_key: Option<FragmentNodeMetadataKey>,
    pub engine: ParsingRuleSet,
}

impl From<SyntaxFragmentBatch> for SyntaxTree {
    fn from(value: SyntaxFragmentBatch) -> Self {
        let mut members = value.fragments;
        members.sort_by(|lhs, rhs| lhs.seq().cmp(&rhs.seq()));

        let mut global_offset = GlobalOffset::default();

        let (children, metadata_entriess): (Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, Vec<StatementMetadataEntry>) = 
            members.into_iter()
            .map(|member| {
                let entry_offset = global_offset.clone();
                global_offset.of_byte += member.statement_byte_len();
                global_offset.of_char += member.statement_char_len();

                let metadata_entry = StatementMetadataEntry{ 
                    global_offset: entry_offset,
                    map: member.metadata_entry.map 
                };

                (rowan::NodeOrToken::Node(member.new_statement), metadata_entry)
            })
            .unzip()
        ;

        let root_metadata_entry = new_root_metadata(value.engine.full_emit_config().from_symbol, global_offset.of_byte, global_offset.of_char);
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
        node_type: parser_core::NodeType::Node, 
        patch: parser_core::PatchAction::None, 
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
    fn apply_batch(&self, batches: SyntaxFragmentBatch) -> Self::Output;
}

pub fn apply_batch(
    root: &rowan::SyntaxNode<RowanLangageImpl>, 
    metadata_table: &MetadataTable, 
    engine: ParsingRuleSet, 
    batch: SyntaxFragmentBatch) -> SyntaxTree
{
    let batch_range = batch.replace_from..(batch.replace_from + batch.replace_size);

    let (children, metadata_entries): (Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, Vec<StatementMetadataEntry>) = 
        batch.fragments.into_iter()
        .map(|SyntaxFragment { new_statement: child, metadata_entry, .. }| {
            (rowan::NodeOrToken::Node(child), metadata_entry)
        })
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
        entry.global_offset.of_byte = global_byte_offset;
        entry.global_offset.of_char = global_char_offset;

        global_byte_offset += usize::from(node.text_len());
        global_char_offset += measure_statement_char_len(std::borrow::Borrow::borrow(node.as_node().unwrap()));
    });

    let root_metadata_entry = new_root_metadata(engine.full_emit_config().from_symbol, global_byte_offset, global_char_offset);
    let new_metadata_table = MetadataTable::new(new_metadata_entries, root_metadata_entry);

    SyntaxTree::new(new_root, new_metadata_table, ParseMode::ByStatement, engine)
}