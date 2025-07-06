#![cfg(not(engine_ungenerated))]

use tolerant_parser_sdk::support::test_support;

mod parser_tests_members {
    mod parse_dispatcher_tests;
    mod node_handler_tests;
    mod parse_recovery_tests;
    mod full_parse_tests;
    mod incremental_parse_tests;
    mod incremental_parse_support_tests;
    mod syntax_tree_tests;
}
