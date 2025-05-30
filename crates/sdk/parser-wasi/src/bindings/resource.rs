use parser_core::syntax_tree::MetadataAccess;
use parser_core::syntax_tree::NodeOperation;
use super::parser_world::exports::ritalin::parser::parsers;
use super::syntax_tree_world::exports::ritalin::parser::syntaxes;
use super::types_world::exports::ritalin::parser::types;

pub struct ParserImpl {
    inner: parser_core::Parser,
}

impl ParserImpl {
    pub fn new(engine: engine_core::Engine) -> Self {
        Self {
            inner: parser_core::Parser::new(engine),
        }
    }
}

impl parsers::GuestParser for ParserImpl {
    fn parse(&self,source: String,) -> parsers::SyntaxTree {
        let tree = self.inner.parse(&source).expect("Failed to parse");
        SyntaxTreeImpl::from_raw(tree)
    }
}

pub struct SyntaxTreeImpl {
    inner: parser_core::syntax_tree::SyntaxTree,
}

impl SyntaxTreeImpl {
    pub fn from_raw(tree: parser_core::syntax_tree::SyntaxTree) -> syntaxes::SyntaxTree {
        syntaxes::SyntaxTree::new(Self { inner: tree })
    }
}

impl syntaxes::GuestSyntaxTree for SyntaxTreeImpl {
    fn root(&self,) -> syntaxes::SyntaxNode {
        SyntaxNodeImpl::from_raw(self.inner.root().clone())
    }
}

pub struct SyntaxNodeImpl {
    inner: parser_core::syntax_tree::SyntaxNode,
}

impl SyntaxNodeImpl {
    pub fn from_raw(node: parser_core::syntax_tree::SyntaxNode) -> syntaxes::SyntaxNode {
        syntaxes::SyntaxNode::new(Self { inner: node })
    }
}

impl syntaxes::GuestSyntaxNode for SyntaxNodeImpl {
    fn metadata_key(&self,) -> syntaxes::MetadataKey {
        self.inner.metadata_key().into()
    }

    fn metadata(&self,) -> syntaxes::Metadata {
        self.inner.metadata().into()
    }

    fn parent(&self,) -> Option<syntaxes::SyntaxNode> {
        self.inner.parent().as_ref().map(|node| SyntaxNodeImpl::from_raw(node.clone()))
    }

    fn children(&self,) -> Vec::<syntaxes::SyntaxElement> {
        self.inner.children()
        .map(|node| node.clone().into())
        .collect()
    }
}

pub struct SyntaxTokenSetImpl {
    inner: parser_core::syntax_tree::SyntaxTokenSet,
}

impl SyntaxTokenSetImpl {
    pub fn from_raw(token_set: parser_core::syntax_tree::SyntaxTokenSet) -> syntaxes::SyntaxTokenSet {
        syntaxes::SyntaxTokenSet::new(Self { inner: token_set })
    }
}

impl syntaxes::GuestSyntaxTokenSet for SyntaxTokenSetImpl {
    fn metadata_key(&self,) -> syntaxes::MetadataKey {
        self.inner.metadata_key().into()
    }

    fn metadata(&self,) -> syntaxes::Metadata {
        self.inner.metadata().into()
    }

    fn parent(&self,) -> Option<syntaxes::SyntaxNode> {
        self.inner.parent().as_ref().map(|node| SyntaxNodeImpl::from_raw(node.clone()))
    }

    fn leading_trivia(&self,) -> Vec::<syntaxes::SyntaxTokenItem> {
        self.inner.leading_trivia()
        .map(|node| SyntaxTokenItemImpl::from_raw(node))
        .collect()
    }

    fn token(&self,) -> syntaxes::SyntaxTokenItem {
        SyntaxTokenItemImpl::from_raw(self.inner.token())
    }

    fn trailing_trivia(&self,) -> Vec::<syntaxes::SyntaxTokenItem> {
        self.inner.trailing_trivia()
        .map(|node| SyntaxTokenItemImpl::from_raw(node))
        .collect()
    }
}

pub struct SyntaxTokenItemImpl {
    inner: parser_core::syntax_tree::SyntaxTokenItem,
}

impl SyntaxTokenItemImpl {
    pub fn from_raw(item: parser_core::syntax_tree::SyntaxTokenItem) -> syntaxes::SyntaxTokenItem {
        syntaxes::SyntaxTokenItem::new(Self { inner: item })
    }
}

impl syntaxes::GuestSyntaxTokenItem for SyntaxTokenItemImpl {
    fn metadata_key(&self,) -> syntaxes::MetadataKey {
        self.inner.metadata_key().into()
    }

    fn metadata(&self,) -> syntaxes::Metadata {
        self.inner.metadata().into()
    }

    fn parent(&self,) -> Option<syntaxes::SyntaxNode> {
        self.inner.parent().as_ref().map(|node| SyntaxNodeImpl::from_raw(node.clone()))
    }

    fn value(&self,) -> String {
        self.inner.value().into()
    }
}

impl From<parser_core::syntax_tree::SyntaxElement> for syntaxes::SyntaxElement {
    fn from(value: parser_core::syntax_tree::SyntaxElement) -> Self {
        match value {
            parser_core::syntax_tree::SyntaxElementDef::Node(node) => {
                syntaxes::SyntaxElement::Node(SyntaxNodeImpl::from_raw(node))
            }
            parser_core::syntax_tree::SyntaxElementDef::TokenSet(token_set) => {
                syntaxes::SyntaxElement::TokenSet(SyntaxTokenSetImpl::from_raw(token_set))
            }
        }
    }
}

impl From<parser_core::NodeMetadataKey> for syntaxes::MetadataKey {
    fn from(value: parser_core::NodeMetadataKey) -> Self {
        Self {
            kind: value.kind.into(),
            offset: value.offset as u32,
            len: value.len as u32,
            is_leaf: value.is_leaf,
        }
    }
}

impl From<parser_core::NodeMetadata> for syntaxes::Metadata {
    fn from(value: parser_core::NodeMetadata) -> Self {
        Self {
            node_type: value.node_type.clone().into(),
            edit_state: value.edit_state as u64,
            patch: value.patch.clone().into(),
            char_offset: value.char_offset as u32,
            char_len: value.char_len as u32,
        }
    }
}

impl From<engine_core::SyntaxKind> for types::SyntaxKind {
    fn from(value: engine_core::SyntaxKind) -> Self {
        Self {
            name: value.text.into(),
            group: value.group.into(),
        }
    }
}

impl From<engine_core::SymbolGroup> for types::SymbolGroup {
    fn from(value: engine_core::SymbolGroup) -> Self {
        match value {
            engine_core::SymbolGroup::Keyword => types::SymbolGroup::Keyword,
            engine_core::SymbolGroup::NonKeyword => types::SymbolGroup::NonKeyword,
            engine_core::SymbolGroup::Pattern => types::SymbolGroup::Pattern,
            engine_core::SymbolGroup::NonTerminal => types::SymbolGroup::NonTerminal,
        }
    }
}

impl From<parser_core::NodeType> for types::NodeType {
    fn from(value: parser_core::NodeType) -> Self {
        match value {
            parser_core::NodeType::Node => types::NodeType::Node,
            parser_core::NodeType::TokenSet => types::NodeType::TokenSet,
            parser_core::NodeType::TokenItem => types::NodeType::TokenItem,
            parser_core::NodeType::LeadingToken => types::NodeType::LeadingTrivia,
            parser_core::NodeType::TrailingToken => types::NodeType::TrailingTrivia,
        }
    }
}

impl From<parser_core::PatchAction> for types::PatchAction {
    fn from(value: parser_core::PatchAction) -> Self {
        match value {
            parser_core::PatchAction::None => types::PatchAction::None,
            parser_core::PatchAction::Delete => types::PatchAction::Delete,
            parser_core::PatchAction::Shift => types::PatchAction::Shift,
            parser_core::PatchAction::Invalid => types::PatchAction::Invalid,
        }
    }
}

pub struct SyntaxTreeComponent;

impl syntaxes::Guest for SyntaxTreeComponent {
    type SyntaxTree = SyntaxTreeImpl;
    type SyntaxNode = SyntaxNodeImpl;
    type SyntaxTokenSet = SyntaxTokenSetImpl;
    type SyntaxTokenItem = SyntaxTokenItemImpl;
}

#[allow(unused)]
pub enum ParserTypesComponent {}