use std::collections::{HashMap, HashSet};

use engine_core::{parser_engine::ParsingRuleSet, SyntaxKind};
use rowan::NodeOrToken;
use crate::{metadata::{StatementMetadataMap}, syntax_tree::RowanLangageImpl, NodeId, NodeMetadata, NodeMetadataKey};

use super::EditScope;


pub struct TreeGardener {
    pub node: rowan::SyntaxNode<RowanLangageImpl>,
}

impl TreeGardener {
    pub fn pick_token(&self, offset: rowan::TextSize) -> Option<FoundToken> {
        match self.node.token_at_offset(offset) {
            rowan::TokenAtOffset::None => return None,
            rowan::TokenAtOffset::Single(token) | rowan::TokenAtOffset::Between(_, token) => {
                Some(FoundToken{ token })
            }
        }
    }

    pub fn common_anscestor(&self, lhs: Option<FoundToken>, rhs: Option<FoundToken>, except_kind: SyntaxKind) -> Option<rowan::SyntaxNode<RowanLangageImpl>> {
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
        
        lca
    }

    pub fn pick_terminate_kind(&self, engine: ParsingRuleSet) -> SyntaxKind {
        let token = self.node.last_token().unwrap();
        
        let kind = match token.next_token() {
            Some(neighbor) => neighbor.parent().map(|x| engine.from_kind_id(x.kind())),
            None => None
        };

        kind.unwrap_or(engine.full_emit_config().to_symbol)
    }

    pub fn replace_with_new_node(
        &self, 
        new_node: rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>,
        anscestor: &rowan::SyntaxNode<RowanLangageImpl>) -> rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>
    {
        let Some(parent) = anscestor.parent() else {
            return new_node;
        };
        let index = anscestor.index();

        let green_node = parent.green().splice_children(index..=index, vec![new_node]);
        rowan::NodeOrToken::Node(parent.replace_with(green_node))
    }

    pub fn merge_metadata_map(
        &self,
        scope: &EditScope,
        old_pair: Option<(rowan::SyntaxNode<RowanLangageImpl>, &HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>)>,
        (new_anscestor, new_metadata): (rowan::GreenNode, HashMap<NodeMetadataKey, (NodeId, NodeMetadata)>),
        global_byte_offset: usize, global_char_offset: usize, local_char_offset: usize,
        engine: ParsingRuleSet) -> StatementMetadataMap
    {
        let mut new_metadata_map = HashMap::from_iter(
            new_metadata.into_iter()
            .map(|(key, (id, metadata))| {
                (key.into_local(global_byte_offset), (id, metadata.into_local(global_char_offset).into_global(local_char_offset)))
            })
        );

        if let Some((old_anscestor, old_metadata)) = old_pair {
            let anscestor_range: std::ops::Range<usize> = old_anscestor.text_range().into();
            let old_char_len = measure_char_len(old_anscestor.green().as_ref());
            let new_char_len = measure_char_len(std::borrow::Borrow::borrow(&new_anscestor));
            let anscestor_path = old_anscestor.ancestors()
                .map(|x| NodeMetadataKey::from_raw_node(&x, engine).into_local(global_byte_offset))
                .collect::<HashSet<_>>()
            ;

            // Phase1: merge metadata except anscestors
            old_metadata.iter()
                .filter(|(key, _)| {
                    !anscestor_path.contains(key)
                })
                .filter_map(|(key, (id, metadata))| match (key.offset, key.len) {
                    (offset, len) if offset + len <= anscestor_range.start => {
                        // Before anscestor nodes descendants
                        Some((key.clone(), (id.clone(), metadata.clone())))
                    }
                    (offset, _) if offset >= anscestor_range.end => {
                        // After anscestor node descendants
                        let key = NodeMetadataKey{ offset: key.offset + scope.new_byte_len - scope.old_byte_len, ..key.clone() };
                        let metadata = NodeMetadata { char_offset: metadata.char_offset + new_char_len - old_char_len, ..metadata.clone() };
                        Some((key, (id.clone(), metadata)))
                    }
                    _ => {
                        // Ignore anscestor node descendants
                        None
                    }
                })
                .for_each(|(key, (id, metadata))| {
                    new_metadata_map.insert(key, (id, metadata));
                })
            ;

            // Phase2: regenerate anscestors metadata
            for node in old_anscestor.ancestors() {                
                let mut key = NodeMetadataKey::from_raw_node(&node, engine).into_local(global_byte_offset);
                let (id, mut metadata) = old_metadata.get(&key).expect("All of nodes need to have a metadata").clone();

                key.len = key.len + scope.new_byte_len - scope.old_byte_len;
                metadata.char_len = metadata.char_len + new_char_len - old_char_len;

                new_metadata_map.insert(key, (id, metadata));
            }
        }

        return StatementMetadataMap {
            byte_offset: global_byte_offset,
            char_offset: global_char_offset,
            map: new_metadata_map,
        };
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


    // /// Extend the existing byte tange to include the neighboring nodes for the specified node.
    // pub fn find_common_anscestor(&self, root: Option<&rowan::SyntaxNode<RowanLangageImpl>>, terminate_symbol: SyntaxKind) -> Option<rowan::SyntaxNode<RowanLangageImpl>> {
    //     let Some(root) = root else {
    //         return None;
    //     };

    //     // Find neighbor tokens
    //     let Some((lhs, rhs)) = extend_to_neighbors_internal(self, root, terminate_symbol, HashSet::from([root.clone()])) else {
    //         return None;
    //     };

    //     // Find Least common anscestor
    //     let left_anscestors = lhs.parent_ancestors().collect::<Vec<_>>();
    //     let right_anscestors = rhs.parent_ancestors().collect::<Vec<_>>();
    //     let lca = left_anscestors.into_iter().rev().zip(right_anscestors.into_iter().rev())
    //         .take_while(|(lhs, rhs)| *lhs == *rhs)
    //         .last()
    //     ;

    //     lca.map(|(node, _)| node)
    // }


// fn extend_to_neighbors_internal(
//     scope: &EditScope, 
//     root: &rowan::SyntaxNode<RowanLangageImpl>, 
//     terminate_symbol: SyntaxKind,
//     needle: HashSet<rowan::SyntaxNode<RowanLangageImpl>>) -> Option<(rowan::SyntaxToken<RowanLangageImpl>, rowan::SyntaxToken<RowanLangageImpl>)> 
// {
//     let range = root.text_range();
//     let lowest_offset: rowan::TextSize = 
//         u32::max(scope.start_byte_offset as u32, range.start().into())
//         .into()
//     ;
//     let highest_offset: rowan::TextSize = 
//         u32::min(
//             (scope.start_byte_offset + usize::max(scope.old_byte_len, scope.new_byte_len)) as u32, 
//             range.end().into()
//         )
//         .into()
//     ;

//     let lhs = {
//         let token = match root.token_at_offset(lowest_offset) {
//             rowan::TokenAtOffset::None => return None,
//             rowan::TokenAtOffset::Single(node) => node,
//             rowan::TokenAtOffset::Between(_, node) => node,
//         };
//         let first_token = token.parent().as_ref()
//             .and_then(|x| x.first_token())
//             .expect("At least, must exist")
//         ;
//         match first_token.prev_token() {
//             Some(sibling) if sibling.parent_ancestors().any(|x| needle.contains(&x)) => {
//                 // Need this needle descendant and except for terminating symbol
//                 sibling
//             }
//             _ => first_token.clone(),
//         }
//     };

//     let rhs = 'right_hand: {
//         let token = match root.token_at_offset(highest_offset) {
//             rowan::TokenAtOffset::None => return None,
//             rowan::TokenAtOffset::Single(node) => node,
//             rowan::TokenAtOffset::Between(_, node) => node,
//         };
//         let last_token = token.parent().as_ref()
//             .and_then(|x| x.last_token())
//             .expect("At least, must exist")
//         ;

//         break 'right_hand match last_token.next_token() {
//             Some(sibling) if sibling.parent_ancestors().any(|x| needle.contains(&x)) => {
//                 // Need this root descendant
//                 if sibling.kind() != terminate_symbol.id { sibling } else { last_token }
//             }
//             _ => last_token.clone(),
//         }
//     };

//     Some((lhs, rhs))
// }
