use std::collections::HashMap;
use engine_core::{parser_engine::ParsingRuleSet, scanner_engine::ScanEvent, SymbolGroup, SyntaxKind};
use rowan::GreenNode;
use scanner_core::Token;

use crate::{event_dispatcher::ParseEvent, metadata::{GlobalOffset, MetadataTable, StatementMetadataEntry}, syntax_tree::SyntaxTree, NodeId, NodeMetadata, NodeMetadataKey, NodeType, ParseMode, PatchAction};

type ActiveIndex = usize;

pub struct SyntaxTreeBuilder {
    element_stack: Vec<Option<(NodeId, StackEntry)>>,
    water_mark: usize,
    all_metadata_map: HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>,
    active_map_index: usize,
    engine: ParsingRuleSet,
    mode: ParseMode,
    prev_id: Option<NodeId>,
}

impl SyntaxTreeBuilder {
    pub fn new(engine: ParsingRuleSet, mode: ParseMode, prev_id: Option<NodeId>) -> Self {
        Self {
            element_stack: Default::default(),
            water_mark: 0,
            all_metadata_map: Default::default(),
            active_map_index: 0,
            engine,
            mode,
            prev_id,
        }
    }

    pub fn add_token_set(&mut self, event: ParseEvent, lookahead: Option<&Token>) -> Result<(), NodeBuildError> {
        let (edit_state, patch) = match event {
            ParseEvent::Shift { edit_state, .. } => (edit_state, PatchAction::None),
            ParseEvent::PatchShift { edit_state, .. } => (edit_state, PatchAction::Shift),
            ParseEvent::Invalid { edit_state, .. } => (edit_state, PatchAction::Invalid),
            _ => return Err(NodeBuildError::TokenSetFailed)
        };

        let Some(lookahead) = lookahead else {
            return Err(NodeBuildError::TokenSetFailed);
        };
        
        let (id, node) = create_token_set(edit_state, lookahead, patch, self.prev_id.clone(), self.active_map_index, &mut self.all_metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.prev_id = Some(id);
        Ok(())
    }

    pub fn add_invisible_token_set(&mut self, event: ParseEvent, lookahead: Option<&Token>) -> Result<(), NodeBuildError> {
        let (edit_state, patch) = match event {
            ParseEvent::PatchDrop { edit_state, .. } => (edit_state, PatchAction::Delete),
            ParseEvent::Invalid { edit_state, .. } => (edit_state, PatchAction::Invalid),
            _ => {
                return Err(NodeBuildError::TokenSetFailed);
            }
        };
        let Some(lookahead) = lookahead else {
            return Err(NodeBuildError::TokenSetFailed);
        };
        
        let (id, node) = create_token_set(edit_state, lookahead, patch, self.prev_id.clone(), self.active_map_index, &mut self.all_metadata_map);
        self.element_stack.push(Some((id, StackEntry::Invisible(node))));
        self.prev_id = Some(id);
        Ok(())
    }

    pub fn add_patch_shift_token_set(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        let ParseEvent::PatchShift { kind, edit_state, .. } = event else {
            return Err(NodeBuildError::TokenSetFailed);
        };

        let top_element = self.element_stack.iter().rev().flatten().next();
        let offset = match get_node_metadata_key(&self.all_metadata_map, top_element) {
            Some(key) => key.offset + key.len,
            None => 0,
        };
        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind, offset, len: 0, value: None },
            trailing_trivia: None,
        };

        let (id, node) = create_token_set(edit_state, &lookahead, PatchAction::Shift, self.prev_id.clone(), self.active_map_index, &mut self.all_metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.prev_id = Some(id);
        Ok(())
    }

    pub fn add_kind_token(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        let ParseEvent::Shift { kind, edit_state, .. } = event else {
            return Err(NodeBuildError::TokenSetFailed);
        };

        let top_element = self.element_stack.iter().rev().flatten().next();
        let offset = match get_node_metadata_key(&self.all_metadata_map, top_element) {
            Some(key) => key.offset + key.len,
            None => 0,
        };
        let len = if kind.group == SymbolGroup::Keyword { kind.text.len() } else { 0 };

        let lookahead = Token{
            leading_trivia: None,
            main: ScanEvent{ kind, offset, len, value: None },
            trailing_trivia: None,
        };

        let (id, node) = create_token_set(edit_state, &lookahead, PatchAction::None, self.prev_id.clone(), self.active_map_index, &mut self.all_metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.prev_id = Some(id);
        Ok(())
    }

    pub fn add_node(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        match event {
            ParseEvent::Reduce { pop_count, .. } | ParseEvent::PatchReduce { pop_count, .. } if pop_count == 0 => {
                self.element_stack.push(None);
                Ok(())
            }
            ParseEvent::Reduce { kind, pop_count, edit_state, .. } => {
                let (id, node) = create_node(kind, edit_state, pop_count, PatchAction::None, self.engine, &mut self.element_stack, self.active_map_index, &mut self.all_metadata_map);
                self.element_stack.push(Some((id, StackEntry::Node(node))));
                self.prev_id = Some(id);
                Ok(())
            }
            ParseEvent::Accept { kind, edit_state, .. } => {
                let (id, node) = create_node(kind, edit_state, self.element_stack.len(), PatchAction::None, self.engine, &mut self.element_stack, self.active_map_index, &mut self.all_metadata_map);
                self.element_stack.push(Some((id, StackEntry::Node(node))));
                self.prev_id = Some(id);
                Ok(())
            }
            ParseEvent::PatchReduce { kind, pop_count, edit_state, .. } => {
                let (id, node) = create_node(kind, edit_state, pop_count, PatchAction::Shift, self.engine, &mut self.element_stack, self.active_map_index, &mut self.all_metadata_map);
                self.element_stack.push(Some((id, StackEntry::Node(node))));
                self.prev_id = Some(id);
                Ok(())
            }
            ParseEvent::Shift { .. } | ParseEvent::Emit { .. } | 
            ParseEvent::Invalid { .. } | ParseEvent::InvalidEmit { .. } |
            ParseEvent::PatchDrop { .. } | ParseEvent::PatchShift { .. } => {
                Err(NodeBuildError::NodeFailed)
            },
        }
    }

    pub fn emit_statement(&mut self, event: ParseEvent) -> Result<(), NodeBuildError> {
        let (kind, edit_state, pop_count) = match event {
            ParseEvent::Emit { kind, edit_state, .. } => {
                let pop_count = self.element_stack.len() - self.water_mark;
                (kind, edit_state, pop_count)
            }
            ParseEvent::InvalidEmit { kind, edit_state, pop_count } => {
                (kind, edit_state, pop_count)
            }
            _ => {
                return Err(NodeBuildError::NodeFailed);
            }
        };
            
        let (id, node) = create_node(kind, edit_state, pop_count, PatchAction::None, self.engine, &mut self.element_stack, self.active_map_index, &mut self.all_metadata_map);
        self.element_stack.push(Some((id, StackEntry::Node(node))));
        self.water_mark = self.element_stack.len();

        if self.mode == ParseMode::ByStatement {
            // switch active metadata map
            self.active_map_index += 1;
        }

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

        let size = if self.mode == ParseMode::Full { 1 } else { self.active_map_index };
        let mut metadata_table = init_statement_metadata_members(size, &self.element_stack, &self.all_metadata_map);
        let mut root_metadata = StatementMetadataEntry::default();

        // Create a root node
        let (_, root) = create_node(kind, edit_state, self.element_stack.len(), PatchAction::None, self.engine, &mut self.element_stack, self.active_map_index, &mut self.all_metadata_map);
        
        self.all_metadata_map.into_iter()
            .for_each(|(_, (index, mut metadata, mut key))| {
                // adjust to the local offset
                let entry = match self.mode {
                    ParseMode::Full => &mut root_metadata,
                    ParseMode::ByStatement if index == self.active_map_index => &mut root_metadata,
                    ParseMode::ByStatement  => &mut metadata_table[index],
                };
                
                key = key.into_local(entry.global_offset.of_byte);
                metadata = metadata.into_local(entry.global_offset.of_char);
                entry.map.insert(key, metadata);
            })
        ;
        Ok(SyntaxTree::new(root, MetadataTable::new(metadata_table, root_metadata), self.mode, self.engine))
    }

    pub fn build_branch(mut self) -> Result<(rowan::GreenNode, HashMap<NodeMetadataKey, NodeMetadata>), NodeBuildError> {
        let len = self.element_stack.len();
        let (_, children) = pop_node_from_stack(&mut self.element_stack, len);
        let Some(rowan::NodeOrToken::Node(node)) = children.first() else {
            return Err(NodeBuildError::EmptyTree);
        };
        let metadata = HashMap::from_iter(self.all_metadata_map.into_iter()
            .map(|(_, (_, metadata, key))| {
                (key, metadata)}
            ));

        Ok((node.clone(), metadata))
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
    state: usize, lookahead: &Token, patch: PatchAction, mut last_id: Option<NodeId>,
    active_index: ActiveIndex,
    metadata_map: &mut HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>) -> (NodeId, rowan::GreenNode)
{
    let mut children: Vec<(NodeId, NodeElement)> = vec![];
    let id = next_node_id();
    let kind = lookahead.main.kind;
    
    'leading_trivia: {
        if let Some(leadings) = lookahead.leading_trivia.as_ref() {
            for trivia in leadings {
                let (child_id, child_node) = create_token_item(state, trivia, NodeType::LeadingToken, patch.clone(), last_id.as_ref(), active_index, metadata_map);
                children.push((child_id, NodeElement::Token(child_node)));
                last_id = Some(child_id);
            }
        }
        break 'leading_trivia;
    }
    'main_token_item: {
        let (child_id, child_node) = create_token_item(state, &lookahead.main, NodeType::TokenItem, patch.clone(), last_id.as_ref(), active_index, metadata_map);
        children.push((child_id, NodeElement::Token(child_node)));
        last_id = Some(child_id);
        break 'main_token_item;
    }
    'trailing_trivia: {
        if let Some(trailings) = lookahead.trailing_trivia.as_ref() {
            for trivia in trailings {
                let (child_id, child_node) = create_token_item(state, trivia, NodeType::TrailingToken, patch.clone(), last_id.as_ref(), active_index, metadata_map);
                children.push((child_id, NodeElement::Token(child_node)));
                last_id = Some(child_id);
            }
        }
        break 'trailing_trivia;
    }

    'node_set: {
        let (child_ids, child_nodes): (Vec<NodeId>, Vec<NodeElement>) = children.into_iter().unzip();

        let element = GreenNode::new(rowan::SyntaxKind(kind.id as u16), child_nodes);
        
        // // resolve offset & len
        let ((offset, len), (char_offset, char_len)) = resolve_token_items_range(child_ids, metadata_map);

        // add metadata
        let metadata_key = NodeMetadataKey{ kind, offset, len, is_leaf: false };
        let metadata = NodeMetadata{ 
            edit_state: state, node_type: NodeType::TokenSet, patch, 
            char_offset, char_len
        };
        metadata_map.insert(id, (active_index, metadata, metadata_key));

        break 'node_set (id, element)
    }
}

fn resolve_token_items_range<'a>(
    child_ids: Vec<NodeId>,
    metadata_map: &HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>) -> ((usize, usize), (usize, usize)) 
{
    let (offset, char_offset) = child_ids.first()
        .and_then(|id| metadata_map.get(&id))
        .map(|(_, metadata, key)| (key.offset, metadata.char_offset))
        .unwrap_or((0, 0))
    ;

    let (len, char_len) = child_ids.into_iter()
        .filter_map(|id| metadata_map.get(&id))
        .fold((0, 0), |(len, char_len), (_, metadata, key)| (len + key.len, char_len + metadata.char_len))
    ;

    ((offset, len), (char_offset, char_len))
}

fn create_token_item(
    state: usize, lookahead: &ScanEvent, node_type: NodeType, patch: PatchAction,
    last_id: Option<&NodeId>,
    active_index: ActiveIndex,
    metadata_map: &mut HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>) -> (NodeId, rowan::GreenToken) 
{
    let id = next_node_id();
    let node = rowan::GreenToken::new(
        rowan::SyntaxKind(lookahead.kind.id as u16), 
        &lookahead.value.clone().unwrap_or_else (|| "".into())
    );

    let (metadata_key, metadata) = create_token_metadata_pair(
        lookahead, state, node_type, patch, 
        last_id.and_then(|id| metadata_map.get(&id))
    );
    metadata_map.insert(id, (active_index, metadata, metadata_key));

    (id, node)
}

fn create_token_metadata_pair(event: &ScanEvent, state: usize, node_type: NodeType, patch: PatchAction, last_metadata: Option<&(ActiveIndex, NodeMetadata, NodeMetadataKey)>) -> (NodeMetadataKey, NodeMetadata) {
    // Eval as utf16 string
    let char_len = event.value.as_ref().map(|s| s.encode_utf16().count()).unwrap_or(0);

    let char_offset = match last_metadata {
        Some((_, metadata, _)) => metadata.char_offset + metadata.char_len,
        None => 0,
    };
    
    let key = NodeMetadataKey{ kind: event.kind, offset: event.offset, len: event.len, is_leaf: true };
    let metadata = NodeMetadata{ 
        edit_state: state, node_type, patch, 
        char_offset, char_len
    };

    (key, metadata)
}

fn create_node(
    kind: SyntaxKind, state: usize, pop_count: usize, patch: PatchAction, 
    engine: ParsingRuleSet,
    element_stack: &mut Vec<Option<(NodeId, StackEntry)>>,
    active_index: ActiveIndex,
    metadata_map: &mut HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>) -> (NodeId, rowan::GreenNode)
{
    let id = next_node_id();
    let (child_ids, mut child_nodes) = pop_node_from_stack(element_stack, pop_count);
    
    remap_alternative_symbol(&mut child_nodes, metadata_map, &child_ids, kind, engine);

    // resolve offset & len
    let ((offset, len), (char_offset, char_len)) = resolve_token_items_range(child_ids, metadata_map);

    let node = rowan::GreenNode::new(rowan::SyntaxKind(kind.id as u16), child_nodes);
    let key = NodeMetadataKey{ kind, offset, len, is_leaf: false };
    let metadata = NodeMetadata{ 
        edit_state: state, node_type: NodeType::Node, patch,
        char_offset, char_len,
    };
    metadata_map.insert(id, (active_index, metadata, key));
    
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
            Some(Some((id, StackEntry::Invisible(element)))) => {
                elements.push((id, NodeElement::Node(element) ));
            }
            _ => {}
        }
        if pop_count == 0 { break }
    }


    while let Some(Some((id, StackEntry::Invisible(element)))) = element_stack.last() {
        elements.push((id.clone(), NodeElement::Node(element.clone())));
        element_stack.pop();
    }

    elements.into_iter().rev().unzip()
}

fn get_node_metadata_key<'a>(
    metadata_map: &'a HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>, 
    node_entry: Option<&'a (NodeId, StackEntry)>) -> Option<&'a NodeMetadataKey> 
{
    if let Some((id, _)) = node_entry {
        return metadata_map.get(id).map(|(_, _, key)| key);
    }
    
    None
}

fn remap_alternative_symbol(
    child_nodes: &mut Vec<rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>>, 
    metadata_map: &mut HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>,
    child_ids: &[NodeId], 
    parent_kind: SyntaxKind, 
    engine: ParsingRuleSet) 
{
    for i in 0..child_nodes.len() {
        let child_kind = engine.from_kind_id(child_nodes[i].kind().0 as u32);

        if let (Some(alt), rowan::NodeOrToken::Node(node)) = (engine.from_alt_symbol(parent_kind, child_kind), child_nodes[i].clone()) {
            if let Some((index, metadata, key)) = metadata_map.remove(&child_ids[i]) {
                let grand_children = node.children()
                    .map(|node| node.to_owned())
                    .collect::<Vec<_>>()
                ;
                let new_key = NodeMetadataKey{kind: *alt, ..key};

                child_nodes[i] = rowan::NodeOrToken::Node(rowan::GreenNode::new(rowan::SyntaxKind(alt.id as u16), grand_children));
                metadata_map.insert(child_ids[i], (index, metadata, new_key));
            }
        }
    }
}

fn init_statement_metadata_members(
    size: usize, 
    elements: &Vec<Option<(NodeId, StackEntry)>>, 
    all_metadata_map: &HashMap<NodeId, (ActiveIndex, NodeMetadata, NodeMetadataKey)>) -> Vec<StatementMetadataEntry> 
{
    let mut table = Vec::with_capacity(size);
    table.resize(size, None);

    // init statement metadata map
    elements.iter()
        .flatten()
        .filter_map(|(id, _)| all_metadata_map.get(id))
        .for_each(|(index, metadata, keys)| {
            table[*index] = Some(StatementMetadataEntry {
                global_offset: GlobalOffset {
                    of_byte: keys.offset,
                    of_char: metadata.char_offset,
                },
                map: Default::default(),
            });
        })
    ;

    if table[0].is_none() {
        table[0] = Some(StatementMetadataEntry {
            global_offset: GlobalOffset::default(),
            map: Default::default(),
        });
    }    

    table.into_iter().flatten().collect()
}

enum StackEntry {
    Node(rowan::GreenNode),
    Invisible(rowan::GreenNode),
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