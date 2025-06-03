use engine_core::SyntaxKind;
use crate::{metadata::StatementMetadataMap, syntax_tree::RowanLangageImpl};


pub struct TreeGardener {
    pub stmt_node: rowan::SyntaxNode<RowanLangageImpl>,
}

impl TreeGardener {
    pub fn left_hand_token_for(&self, offset: rowan::TextSize) -> Option<FoundToken> {
        todo!()
    }

    pub fn right_hand_token_for(&self, offset: rowan::TextSize) -> Option<FoundToken> {
        todo!()
    }

    pub fn common_anscestor(&self, lhs: Option<FoundToken>, rhs: Option<FoundToken>, except_kind: SyntaxKind) -> Option<rowan::SyntaxNode<RowanLangageImpl>> {
        todo!()
    }

    pub fn replace_with_new_node(
        &self, 
        new_node: rowan::GreenNode,
        anscestor: &rowan::SyntaxNode<RowanLangageImpl>) -> rowan::NodeOrToken<rowan::GreenNode, rowan::GreenToken>
    {
        todo!()
    }

    pub fn merge_metadata_map(
        &self,
        anscestor: rowan::SyntaxNode<RowanLangageImpl>,
        old_metadata: Option<&StatementMetadataMap>,
        new_metadata: StatementMetadataMap) -> StatementMetadataMap
    {
        if let Some(old_metadata) = old_metadata {
            // Skip for a new statement
            return new_metadata;
        }
        todo!()
    }
}

pub struct FoundToken {
    pub token: rowan::SyntaxToken<RowanLangageImpl>
}

impl FoundToken {
    pub fn prev_token(
        &self,
        stmt: &rowan::SyntaxNode<RowanLangageImpl>, 
        except_kind: SyntaxKind) -> Option<rowan::SyntaxToken<RowanLangageImpl>> 
    {
        todo!()
    }

    pub fn next_token(
        &self,
        stmt: &rowan::SyntaxNode<RowanLangageImpl>, 
        except_kind: SyntaxKind) -> Option<rowan::SyntaxToken<RowanLangageImpl>> 
    {
        todo!()
    }
}

fn is_descendant(
    stmt: &rowan::SyntaxNode<RowanLangageImpl>, 
    node: Option<&rowan::SyntaxNode<RowanLangageImpl>>) -> bool 
{
    todo!()
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
