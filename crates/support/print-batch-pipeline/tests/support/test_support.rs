use parser_core::{support::test_support::{ExpectMetadataKey, ExpectMetadataValue}, syntax_tree::MetadataAccess};
use print_batch_pipeline::{PrintBatch, PrintBatchPipeline, SlotSizeHint};

#[derive(Debug, serde::Deserialize)]
pub enum ExpectPrintBatch {
    Node {
        kind_name: String,
        metadata_key: ExpectMetadataKey,
        metadata_obj: ExpectMetadataValue,
        value: Option<String>,
        depth: usize,
    },
    BeginBatch { slot_offset: usize, slot_new_size: usize, slot_old_size: usize },
    EndBatch,
    Source(String),
}

pub fn verify(pipeline: PrintBatchPipeline, expect_batches: &[ExpectPrintBatch]) {
    let mut count = 0;
    
    for (batch, expect_batch) in pipeline.into_iter().zip(expect_batches) {
        count += 1;

        match (&batch, expect_batch) {
            (PrintBatch::Node(actual), ExpectPrintBatch::Node { kind_name, metadata_key: expect_key, metadata_obj: expect_metadata, value: expect_value, depth: expect_depth  }) => {
                assert_eq!(kind_name, actual.metadata_key().kind.text);
                assert_eq!(*expect_key, ExpectMetadataKey::from(actual.metadata_key()), "(node: {:?}#{}, kind: {})", expect_metadata.node_type, expect_depth, kind_name);
                assert_eq!(*expect_metadata, ExpectMetadataValue::from(actual.metadata()), "(node: {:?}#{}, kind: {})", expect_metadata.node_type, expect_depth, kind_name);
                assert_eq!(*expect_value, actual.value(), "(node: {:?}#{}, kind: {})", expect_metadata.node_type, expect_depth, kind_name);
                assert_eq!(*expect_depth, actual.depth(), "(node: {:?}#{}, kind: {})", expect_metadata.node_type, expect_depth, kind_name);              
            }
            (PrintBatch::BeginBatch(actual), ExpectPrintBatch::BeginBatch { slot_offset: expect_offset, slot_new_size: expect_new_size, slot_old_size: expect_old_size }) => {
                let expect_slot = SlotSizeHint{
                    offset: *expect_offset,
                    new_size: *expect_new_size,
                    old_size: *expect_old_size,
                };
                assert_eq!(expect_slot, *actual);
            }
            (PrintBatch::EndBatch, ExpectPrintBatch::EndBatch) => {
                // skip
            }
            (PrintBatch::Source { text: actual_str, .. }, ExpectPrintBatch::Source(expect_str)) => {
                assert_eq!(expect_str,actual_str);
            }
            _ => {
                panic!("Unmatched batch type \n  actual: {:?}, \n  expect: {:?})", batch, expect_batch)
            }
        }
    }

    assert_eq!(expect_batches.len(), count);
}