use std::collections::HashMap;
use engine_core::{parser_engine::ParsingRuleSet, scanner_engine::ScanEvent, SymbolGroup, SyntaxKind};
use rowan::GreenNode;
use scanner_core::Token;

use crate::{event_dispatcher::ParseEvent, syntax_tree::SyntaxTree, NodeId, NodeMetadata, NodeMetadataKey, NodeType};

pub struct SyntaxTreeBuilder {
    element_stack: Vec<Option<(NodeId, StackEntry)>>,
    water_mark: usize,
    metadata_map: HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>,
    engine: ParsingRuleSet,
    prev_id: Option<NodeId>,
}

impl SyntaxTreeBuilder {
    pub fn new(engine: ParsingRuleSet, prev_id: Option<NodeId>) -> Self {
        Self {
            element_stack: Default::default(),
            water_mark: 0,
            metadata_map: Default::default(),
            engine,
            prev_id,
        }
    }

    pub fn add_token_set(&mut self, event: ParseEvent, lookahead: Option<&Token>) -> Result<(), NodeBuildError> {
        let ParseEvent::Shift { edit_state, .. } = event else {
            return Err(NodeBuildError::TokenSetFailed);
        };
        let Some(lookahead) = lookahead else {
            return Err(NodeBuildError::TokenSetFailed);
        };
        
        let (id, node) = create_token_set(edit_state, lookahead, self.prev_id.clone(), &mut self.metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.prev_id = Some(id);
        Ok(())
    }

    pub fn add_kind_token(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        let ParseEvent::Shift { kind, edit_state, .. } = event else {
            return Err(NodeBuildError::TokenSetFailed);
        };

        let offset = match get_node_metadata_key(&self.metadata_map, self.element_stack.last()) {
            Some(key) => key.offset + key.len,
            None => 0,
        };
        let len = if kind.group == SymbolGroup::Keyword { kind.text.len() } else { 0 };

        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind, offset, len, value: None },
            trailing_trivia: None,
        };

        let (id, node) = create_token_set(edit_state, &lookahead, self.prev_id.clone(), &mut self.metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.prev_id = Some(id);
        Ok(())
    }

    pub fn add_node(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        match event {
            ParseEvent::Reduce { pop_count, .. } if pop_count == 0 => {
                self.element_stack.push(None);
                Ok(())
            }
            ParseEvent::Reduce { kind, pop_count, edit_state, .. } => {
                let (id, node) = create_node(kind, edit_state, pop_count, self.engine, &mut self.element_stack, &mut self.metadata_map);
                self.element_stack.push(Some((id, StackEntry::Node(node))));
                self.prev_id = Some(id);
                Ok(())
            }
            ParseEvent::Accept { kind, edit_state, .. } => {
                let (id, node) = create_node(kind, edit_state, self.element_stack.len(), self.engine, &mut self.element_stack, &mut self.metadata_map);
                self.element_stack.push(Some((id, StackEntry::Node(node))));
                self.prev_id = Some(id);
                Ok(())
            }
            ParseEvent::Shift { .. } | ParseEvent::Emit { .. } => {
                Err(NodeBuildError::NodeFailed)
            },
        }
    }

    pub fn emit_statement(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        let ParseEvent::Emit { kind, edit_state, .. } = event else {
            return Err(NodeBuildError::NodeFailed);
        };
        let pop_count = self.element_stack.len() - self.water_mark;
        let (id, node) = create_node(kind, edit_state, pop_count, self.engine, &mut self.element_stack, &mut self.metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.water_mark = self.element_stack.len();

        Ok(())
    }

    /// Build syntax tree
    /// 
    /// # Remarks
    /// * `accept_event` must be ParseEvent::Accept variant.
    /// * At least, it must be added child node/nodeset.
    pub fn build(mut self, accept_event: ParseEvent) -> Result<SyntaxTree, NodeBuildError> {
        let ParseEvent::Accept { kind, edit_state, .. } = accept_event else {
            return Err(NodeBuildError::NodeFailed);
        };
        if self.element_stack.is_empty() {
            return Err(NodeBuildError::EmptyTree);
        }

        let (_, root) = create_node(kind, edit_state, self.element_stack.len(), self.engine, &mut self.element_stack, &mut self.metadata_map);

        let metadata_map = HashMap::<NodeMetadataKey, (NodeId, NodeMetadata)>::from_iter(
            self.metadata_map.into_iter().map(|(id, (metadata, key))| (key, (id, metadata)))
        );
        Ok(SyntaxTree::new(root, metadata_map, self.engine))
    }
}


thread_local! {
    static ID_GENERATOR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
}

fn next_node_id() -> NodeId {
    let ts = std::time::Instant::now();
    let id = ID_GENERATOR.with(|g| g.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    (ts, id)
}

type NodeElement = rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>;
fn create_token_set(
    state: usize, lookahead: &Token, mut last_id: Option<NodeId>,
    metadata_map: &mut HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>) -> (NodeId, rowan::GreenNode)
{
    let mut children: Vec<(NodeId, NodeElement)> = vec![];
    let id = next_node_id();
    let kind = lookahead.main.kind;
    
    'leading_trivia: {
        if let Some(leadings) = lookahead.leading_trivia.as_ref() {
            for trivia in leadings {
                let (child_id, child_node) = create_token_item(state, trivia, NodeType::LeadingToken, last_id.as_ref(), metadata_map);
                children.push((child_id, NodeElement::Token(child_node)));
                last_id = Some(child_id);
            }
        }
        break 'leading_trivia;
    }
    'main_token_item: {
        let (child_id, child_node) = create_token_item(state, &lookahead.main, NodeType::TokenItem, last_id.as_ref(), metadata_map);
        children.push((child_id, NodeElement::Token(child_node)));
        last_id = Some(child_id);
        break 'main_token_item;
    }
    'trailing_irivia: {
        if let Some(trailings) = lookahead.trailing_trivia.as_ref() {
            for trivia in trailings {
                let (child_id, child_node) = create_token_item(state, trivia, NodeType::TrailingToken, last_id.as_ref(), metadata_map);
                children.push((child_id, NodeElement::Token(child_node)));
                last_id = Some(child_id);
            }
        }
        break 'trailing_irivia;
    }

    'node_set: {
        let (child_ids, child_nodes): (Vec<NodeId>, Vec<NodeElement>) = children.into_iter().unzip();

        let element = GreenNode::new(rowan::SyntaxKind(kind.id as u16), child_nodes);
        
        // // resolve offset & len
        let ((offset, len), (char_offset, char_len)) = resolve_token_items_range(child_ids, metadata_map);

        // add metadata
        let metadata_key = NodeMetadataKey{ kind, offset, len, is_leaf: false };
        let metadata = NodeMetadata{ 
            edit_state: state, node_type: NodeType::TokenSet, recovery: None, 
            char_offset, char_len
        };
        metadata_map.insert(id, (metadata, metadata_key));

        break 'node_set (id, element)
    }
}

fn resolve_token_items_range<'a>(
    child_ids: Vec<NodeId>,
    metadata_map: &HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>) -> ((usize, usize), (usize, usize)) 
{
    let (offset, char_offset) = child_ids.first()
        .and_then(|id| metadata_map.get(&id))
        .map(|(metadata, key)| (key.offset, metadata.char_offset))
        .unwrap_or((0, 0))
    ;

    let (len, char_len) = child_ids.into_iter()
        .filter_map(|id| metadata_map.get(&id))
        .fold((0, 0), |(len, char_len), (metadata, key)| (len + key.len, char_len + metadata.char_len))
    ;

    ((offset, len), (char_offset, char_len))
}

fn create_token_item(
    state: usize, lookahead: &ScanEvent, node_type: NodeType,
    last_id: Option<&NodeId>,
    metadata_map: &mut HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>) -> (NodeId, rowan::GreenToken) 
{
    let id = next_node_id();
    let node = rowan::GreenToken::new(
        rowan::SyntaxKind(lookahead.kind.id as u16), 
        &lookahead.value.clone().unwrap_or_else (|| "".into())
    );

    let (metadata_key, metadata) = create_token_metadata_pair(lookahead, state, node_type, last_id.and_then(|id| metadata_map.get(&id)));
    metadata_map.insert(id, (metadata, metadata_key));

    (id, node)
}

fn create_token_metadata_pair(event: &ScanEvent, state: usize, node_type: NodeType, last_metadata: Option<&(NodeMetadata, NodeMetadataKey)>) -> (NodeMetadataKey, NodeMetadata) {
    let char_len = event.value.as_ref().map(|s| s.chars().count()).unwrap_or(0);

    let char_offset = match last_metadata {
        Some((metadata, _)) => metadata.char_offset + metadata.char_len,
        None => 0,
    };
    
    // add metadata
    let key = NodeMetadataKey{ kind: event.kind, offset: event.offset, len: event.len, is_leaf: true };
    let metadata = NodeMetadata{ 
        edit_state: state, node_type, recovery: None, 
        char_offset, char_len
    };

    (key, metadata)
}

fn create_node(
    kind: SyntaxKind, state: usize, pop_count: usize, engine: ParsingRuleSet,
    element_stack: &mut Vec<Option<(NodeId, StackEntry)>>,
    metadata_map: &mut HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>) -> (NodeId, rowan::GreenNode)
{
    let id = next_node_id();
    let (child_ids, mut child_nodes) = pop_node_from_stack(element_stack, pop_count);
    
    remap_alternative_symbol(&mut child_nodes, metadata_map, &child_ids, kind, engine);

    // resolve offset & len
    let ((offset, len), (char_offset, char_len)) = resolve_token_items_range(child_ids, metadata_map);

    let node = rowan::GreenNode::new(rowan::SyntaxKind(kind.id as u16), child_nodes);
    let key = NodeMetadataKey{ kind, offset, len, is_leaf: false };
    let metadata = NodeMetadata{ 
        edit_state: state, node_type: NodeType::Node, recovery: None,
        char_offset, char_len,
    };
    metadata_map.insert(id, (metadata, key));
    
    (id, node)
}

fn pop_node_from_stack(element_stack: &mut Vec<Option<(NodeId, StackEntry)>>, mut pop_count: usize) -> (Vec<NodeId>, Vec<NodeElement>) {
    assert!(pop_count <= element_stack.len(), "pop_count: {}, stack/len: {}", pop_count, element_stack.len());
    let mut elements = Vec::with_capacity(pop_count + 1);

    while pop_count > 0 {
        match element_stack.pop() {
            Some(Some((id, StackEntry::Node(element)))) => {
                elements.push((id, NodeElement::Node(element)));
                pop_count -= 1;
            }
            Some(None) => {
                pop_count -= 1;
            }
            Some(Some((id, StackEntry::DeleteRecovery(element)))) => {
                elements.push((id, NodeElement::Node(element) ));
            }
            _ => {}
        }
        if pop_count == 0 { break }
    }


    if let Some(Some((id, StackEntry::DeleteRecovery(element)))) = element_stack.last() {
        elements.push((id.clone(), NodeElement::Node(element.clone())));
        element_stack.pop();
    }

    elements.into_iter().rev().unzip()
}

fn get_node_metadata_key<'a>(
    metadata_map: &'a HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>, 
    node_entry: Option<&'a Option<(NodeId, StackEntry)>>) -> Option<&'a NodeMetadataKey> 
{
    if let Some(Some((id, _))) = node_entry {
        return metadata_map.get(id).map(|(_, key)| key);
    }
    
    None
}

fn remap_alternative_symbol(
    child_nodes: &mut Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, 
    metadata_map: &mut HashMap<NodeId, (NodeMetadata, NodeMetadataKey)>,
    child_ids: &[NodeId], 
    parent_kind: SyntaxKind, 
    engine: ParsingRuleSet) 
{
    for i in 0..child_nodes.len() {
        let child_kind = engine.from_kind_id(child_nodes[i].kind().0 as u32);

        if let (Some(alt), rowan::NodeOrToken::Node(node)) = (engine.from_alt_symbol(parent_kind, child_kind), child_nodes[i].clone()) {
            if let Some((metadata, key)) = metadata_map.remove(&child_ids[i]) {
                let grand_children = node.children()
                    .map(|node| node.to_owned())
                    .collect::<Vec<_>>()
                ;
                let new_key = NodeMetadataKey{kind: *alt, ..key};

                child_nodes[i] = rowan::NodeOrToken::Node(rowan::GreenNode::new(rowan::SyntaxKind(alt.id as u16), grand_children));
                metadata_map.insert(child_ids[i], (metadata, new_key));
            }
        }
    }
}

enum StackEntry {
    Node(rowan::GreenNode),
    DeleteRecovery(rowan::GreenNode),
}

#[derive(PartialEq, Debug, thiserror::Error)]
pub enum NodeBuildError {
    /// At least, It needs node or node set
    #[error("At least, It needs node or node set")]
    EmptyTree,
    /// TokenSet is only acceped Shift event
    #[error("TokenSet is only acceped Shift event")]
    TokenSetFailed,
    /// Node is accepted Reduce or Accept event
    #[error("Node is accepted Reduce or Accept event")]
    NodeFailed,
}