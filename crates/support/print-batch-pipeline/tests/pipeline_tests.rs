
#[path = "support/test_support.rs"]
mod test_support;

mod single_batch {
    use parser_core::{incremental::EditScope, ParseMode, Parser, ParserConfig, RecoveryPenalty};
    use print_batch_pipeline::PrintBatchPipeline;
    use crate::test_support;

    #[test]
    fn test_single_fragment_by_inserting() -> Result<(), anyhow::Error> {
        let source = "SELECT 42 AS x;";
        let new_source = "SELECT 42, 1 AS x;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 10,
            old_byte_len: 0,
            new_byte_len: 4,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };
        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;

        let pipeline = PrintBatchPipeline::new(tree, batches);
        assert_eq!(1, pipeline.batch_size());

        let expect_batches = serde_json::from_str::<Vec<test_support::ExpectPrintBatch>>(include_str!("./fixtures/pipeline_tests/test_single_fragment_by_inserting.json"))?;
        test_support::verify(pipeline, &expect_batches);

        Ok(())
    }

    // FIXME: fn test_single_fragment_by_deleting() -> Result<(), anyhow::Error> {}

    #[test]
    fn test_multi_fragment_by_inserting() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;SELECT 9;";
        let new_source = "SELECT 1;SELECT 2;SELECT 9;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 9,
            old_byte_len: 0,
            new_byte_len: 9,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };
        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;

        let pipeline = PrintBatchPipeline::new(tree, batches);
        assert_eq!(1, pipeline.batch_size());

        let expect_batches = serde_json::from_str::<Vec<test_support::ExpectPrintBatch>>(include_str!("./fixtures/pipeline_tests/test_multi_fragment_by_inserting.json"))?;
        test_support::verify(pipeline, &expect_batches);

        Ok(())
    }

    #[test]
    fn test_multi_fragment_by_inserting_preceding_eof() -> Result<(), anyhow::Error> {
        let source = "SELECT 1;";
        let new_source = "SELECT 1;SELECT 2;";

        let engine = sqlite_engine::create()?;
        let parser = Parser::new(engine.clone());
        let tree = parser.parse(source)?;

        let scope = EditScope{
            start_byte_offset: 9,
            old_byte_len: 0,
            new_byte_len: 9,
        };
        let config = ParserConfig{
            mode: ParseMode::ByStatement,
            penalty: RecoveryPenalty::default(),
        };
        let batches = parser.incremental(&tree, scope).parse_with_config(new_source, config)?;

        let pipeline = PrintBatchPipeline::new(tree, batches);
        assert_eq!(1, pipeline.batch_size());

        let expect_batches = serde_json::from_str::<Vec<test_support::ExpectPrintBatch>>(include_str!("./fixtures/pipeline_tests/test_multi_fragment_by_inserting.json"))?;
        test_support::verify(pipeline, &expect_batches);

        Ok(())
    }

    // FIXME: fn test_multi_fragment_by_deleting() -> Result<(), anyhow::Error> {}
    // FIXME: fn test_multi_fragment_by_inserting() -> Result<(), anyhow::Error> {}

    // FIXME: fn test_single_fragment_by_joining() -> Result<(), anyhow::Error> {}
    // FIXME: fn test_multifragment_by_splitting() -> Result<(), anyhow::Error> {}
}
