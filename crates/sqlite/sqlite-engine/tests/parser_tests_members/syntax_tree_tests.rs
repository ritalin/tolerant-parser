    use parser_core::{Parser, syntax_tree::MetadataAccess, NodeMetadataKey};
    use sqlite_engine::syntax_kind;


mod operation_tests {

    use parser_core::syntax_tree::NodeOperation;

    use super::*;

    #[test]
    fn test_move_sigling_token() -> Result<(), anyhow::Error> {
        let source = "SELECT 1 AS a, 42;";
    
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let Some(token) = tree.root().token_at_offset(0) else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::SELECT, offset: 0, len: 6, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.next_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 6, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.next_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 7, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.next_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 8, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.next_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::r#AS, offset: 9, len: 2, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());


        let Some(token) = token.prev_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 8, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.prev_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::INTEGER, offset: 7, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.prev_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::SPACE, offset: 6, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(token) = token.prev_sibling() else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::SELECT, offset: 0, len: 6, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        assert_eq!(None, token.prev_sibling());

        Ok(())
    }

    #[test]
    fn test_move_sigling_node() -> Result<(), anyhow::Error> {
        let source = "SELECT 1 AS a, 42;";
    
        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine);
        let tree = parser.parse(source)?;

        let Some(token) = tree.root().token_at_offset(13) else { panic!("Token must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::COMMA, offset: 13, len: 1, is_leaf: true };
        assert_eq!(expect_key, token.metadata_key());

        let Some(node) = token.parent() else { panic!("TokenSet must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::COMMA, offset: 13, len: 2, is_leaf: false };
        assert_eq!(expect_key, node.metadata_key());

        assert_eq!(None, node.next_sibling());

        let Some(node) = node.prev_sibling() else { panic!("Node must exist") };
        let expect_key = NodeMetadataKey{ kind: syntax_kind::selcollist, offset: 7, len: 6, is_leaf: false };
        assert_eq!(expect_key, node.metadata_key());

        assert_eq!(None, node.prev_sibling());

        Ok(())
    }
}