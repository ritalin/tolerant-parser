mod scan_rule_map {
  use engine_core::scanner_engine::ScanPattern;
  pub static LEXME_SCAN_RULE: phf::Map<char, &'static [ScanPattern]> = phf::phf_map!{
    '!' => &[
      ScanPattern { id : 77u32 , pattern : "!=" , len : 2usize } ,
    ],
    '%' => &[
      ScanPattern { id : 135u32 , pattern : "%" , len : 1usize } ,
    ],
    '&' => &[
      ScanPattern { id : 127u32 , pattern : "&" , len : 1usize } ,
    ],
    '(' => &[
      ScanPattern { id : 38u32 , pattern : "(" , len : 1usize } ,
    ],
    ')' => &[
      ScanPattern { id : 41u32 , pattern : ")" , len : 1usize } ,
    ],
    '*' => &[
      ScanPattern { id : 133u32 , pattern : "*" , len : 1usize } ,
    ],
    '+' => &[
      ScanPattern { id : 131u32 , pattern : "+" , len : 1usize } ,
    ],
    ',' => &[
      ScanPattern { id : 46u32 , pattern : "," , len : 1usize } ,
    ],
    '-' => &[
      ScanPattern { id : 137u32 , pattern : "->" , len : 2usize } ,
      ScanPattern { id : 132u32 , pattern : "-" , len : 1usize } ,
    ],
    '.' => &[
      ScanPattern { id : 217u32 , pattern : "." , len : 1usize } ,
    ],
    '/' => &[
      ScanPattern { id : 134u32 , pattern : "/" , len : 1usize } ,
    ],
    ';' => &[
      ScanPattern { id : 4u32 , pattern : ";" , len : 1usize } ,
    ],
    '<' => &[
      ScanPattern { id : 129u32 , pattern : "<<" , len : 2usize } ,
      ScanPattern { id : 77u32 , pattern : "<>" , len : 2usize } ,
      ScanPattern { id : 80u32 , pattern : "<=" , len : 2usize } ,
      ScanPattern { id : 81u32 , pattern : "<" , len : 1usize } ,
    ],
    '=' => &[
      ScanPattern { id : 78u32 , pattern : "=" , len : 1usize } ,
    ],
    '>' => &[
      ScanPattern { id : 130u32 , pattern : ">>" , len : 2usize } ,
      ScanPattern { id : 82u32 , pattern : ">=" , len : 2usize } ,
      ScanPattern { id : 79u32 , pattern : ">" , len : 1usize } ,
    ],
    '?' => &[
      ScanPattern { id : 249u32 , pattern : "?" , len : 1usize } ,
    ],
    'a' => &[
      ScanPattern { id : 54u32 , pattern : "ANALYZE" , len : 7usize } ,
      ScanPattern { id : 121u32 , pattern : "ALWAYS" , len : 6usize } ,
      ScanPattern { id : 52u32 , pattern : "ACTION" , len : 6usize } ,
      ScanPattern { id : 56u32 , pattern : "ATTACH" , len : 6usize } ,
      ScanPattern { id : 279u32 , pattern : "ALTER" , len : 5usize } ,
      ScanPattern { id : 51u32 , pattern : "ABORT" , len : 5usize } ,
      ScanPattern { id : 53u32 , pattern : "AFTER" , len : 5usize } ,
      ScanPattern { id : 197u32 , pattern : "ALL" , len : 3usize } ,
      ScanPattern { id : 281u32 , pattern : "ADD" , len : 3usize } ,
      ScanPattern { id : 55u32 , pattern : "ASC" , len : 3usize } ,
      ScanPattern { id : 68u32 , pattern : "AND" , len : 3usize } ,
      ScanPattern { id : 43u32 , pattern : "AS" , len : 2usize } ,
    ],
    'b' => &[
      ScanPattern { id : 73u32 , pattern : "BETWEEN" , len : 7usize } ,
      ScanPattern { id : 57u32 , pattern : "BEFORE" , len : 6usize } ,
      ScanPattern { id : 11u32 , pattern : "BEGIN" , len : 5usize } ,
      ScanPattern { id : 58u32 , pattern : "BY" , len : 2usize } ,
    ],
    'c' => &[
      ScanPattern { id : 125u32 , pattern : "CURRENT_TIMESTAMP" , len : 17usize } ,
      ScanPattern { id : 125u32 , pattern : "CURRENT_DATE" , len : 12usize } ,
      ScanPattern { id : 125u32 , pattern : "CURRENT_TIME" , len : 12usize } ,
      ScanPattern { id : 154u32 , pattern : "CONSTRAINT" , len : 10usize } ,
      ScanPattern { id : 61u32 , pattern : "CONFLICT" , len : 8usize } ,
      ScanPattern { id : 110u32 , pattern : "CURRENT" , len : 7usize } ,
      ScanPattern { id : 138u32 , pattern : "COLLATE" , len : 7usize } ,
      ScanPattern { id : 59u32 , pattern : "CASCADE" , len : 7usize } ,
      ScanPattern { id : 308u32 , pattern : "COLUMN" , len : 6usize } ,
      ScanPattern { id : 19u32 , pattern : "COMMIT" , len : 6usize } ,
      ScanPattern { id : 33u32 , pattern : "CREATE" , len : 6usize } ,
      ScanPattern { id : 164u32 , pattern : "CHECK" , len : 5usize } ,
      ScanPattern { id : 85u32 , pattern : "COLUMN" , len : 6usize } ,
      ScanPattern { id : 146u32 , pattern : "CROSS" , len : 5usize } ,
      ScanPattern { id : 255u32 , pattern : "CASE" , len : 4usize } ,
      ScanPattern { id : 60u32 , pattern : "CAST" , len : 4usize } ,
    ],
    'd' => &[
      ScanPattern { id : 177u32 , pattern : "DEFERRABLE" , len : 10usize } ,
      ScanPattern { id : 214u32 , pattern : "DISTINCT" , len : 8usize } ,
      ScanPattern { id : 16u32 , pattern : "DEFERRED" , len : 8usize } ,
      ScanPattern { id : 62u32 , pattern : "DATABASE" , len : 8usize } ,
      ScanPattern { id : 155u32 , pattern : "DEFAULT" , len : 7usize } ,
      ScanPattern { id : 174u32 , pattern : "DELETE" , len : 6usize } ,
      ScanPattern { id : 64u32 , pattern : "DETACH" , len : 6usize } ,
      ScanPattern { id : 189u32 , pattern : "DROP" , len : 4usize } ,
      ScanPattern { id : 63u32 , pattern : "DESC" , len : 4usize } ,
      ScanPattern { id : 86u32 , pattern : "DO" , len : 2usize } ,
    ],
    'e' => &[
      ScanPattern { id : 18u32 , pattern : "EXCLUSIVE" , len : 9usize } ,
      ScanPattern { id : 116u32 , pattern : "EXCLUDE" , len : 7usize } ,
      ScanPattern { id : 198u32 , pattern : "EXCEPT" , len : 6usize } ,
      ScanPattern { id : 7u32 , pattern : "EXPLAIN" , len : 7usize } ,
      ScanPattern { id : 36u32 , pattern : "EXISTS" , len : 6usize } ,
      ScanPattern { id : 83u32 , pattern : "ESCAPE" , len : 6usize } ,
      ScanPattern { id : 261u32 , pattern : "ELSE" , len : 4usize } ,
      ScanPattern { id : 65u32 , pattern : "EACH" , len : 4usize } ,
      ScanPattern { id : 20u32 , pattern : "END" , len : 3usize } ,
    ],
    'f' => &[
      ScanPattern { id : 111u32 , pattern : "FOLLOWING" , len : 9usize } ,
      ScanPattern { id : 183u32 , pattern : "FOREIGN" , len : 7usize } ,
      ScanPattern { id : 307u32 , pattern : "FILTER" , len : 6usize } ,
      ScanPattern { id : 108u32 , pattern : "FIRST" , len : 5usize } ,
      ScanPattern { id : 311u32 , pattern : "FALSE" , len : 5usize } ,
      ScanPattern { id : 220u32 , pattern : "FROM" , len : 4usize } ,
      ScanPattern { id : 146u32 , pattern : "FULL" , len : 4usize } ,
      ScanPattern { id : 66u32 , pattern : "FAIL" , len : 4usize } ,
      ScanPattern { id : 87u32 , pattern : "FOR" , len : 3usize } ,
    ],
    'g' => &[
      ScanPattern { id : 120u32 , pattern : "GENERATED" , len : 9usize } ,
      ScanPattern { id : 117u32 , pattern : "GROUPS" , len : 6usize } ,
      ScanPattern { id : 232u32 , pattern : "GROUP" , len : 5usize } ,
      ScanPattern { id : 72u32 , pattern : "GLOB" , len : 4usize } ,
    ],
    'h' => &[
      ScanPattern { id : 233u32 , pattern : "HAVING" , len : 6usize } ,
    ],
    'i' => &[
      ScanPattern { id : 199u32 , pattern : "INTERSECT" , len : 9usize } ,
      ScanPattern { id : 17u32 , pattern : "IMMEDIATE" , len : 9usize } ,
      ScanPattern { id : 89u32 , pattern : "INITIALLY" , len : 9usize } ,
      ScanPattern { id : 142u32 , pattern : "INDEXED" , len : 7usize } ,
      ScanPattern { id : 90u32 , pattern : "INSTEAD" , len : 7usize } ,
      ScanPattern { id : 172u32 , pattern : "INSERT" , len : 6usize } ,
      ScanPattern { id : 75u32 , pattern : "ISNULL" , len : 6usize } ,
      ScanPattern { id : 88u32 , pattern : "IGNORE" , len : 6usize } ,
      ScanPattern { id : 263u32 , pattern : "INDEX" , len : 5usize } ,
      ScanPattern { id : 146u32 , pattern : "INNER" , len : 5usize } ,
      ScanPattern { id : 241u32 , pattern : "INTO" , len : 4usize } ,
      ScanPattern { id : 34u32 , pattern : "IF" , len : 2usize } ,
      ScanPattern { id : 69u32 , pattern : "IS" , len : 2usize } ,
      ScanPattern { id : 74u32 , pattern : "IN" , len : 2usize } ,
    ],
    'j' => &[
      ScanPattern { id : 226u32 , pattern : "JOIN" , len : 4usize } ,
    ],
    'k' => &[
      ScanPattern { id : 92u32 , pattern : "KEY" , len : 3usize } ,
    ],
    'l' => &[
      ScanPattern { id : 234u32 , pattern : "LIMIT" , len : 5usize } ,
      ScanPattern { id : 109u32 , pattern : "LAST" , len : 4usize } ,
      ScanPattern { id : 146u32 , pattern : "LEFT" , len : 4usize } ,
      ScanPattern { id : 72u32 , pattern : "LIKE" , len : 4usize } ,
    ],
    'm' => &[
      ScanPattern { id : 122u32 , pattern : "MATERIALIZED" , len : 12usize } ,
      ScanPattern { id : 71u32 , pattern : "MATCH" , len : 5usize } ,
    ],
    'n' => &[
      ScanPattern { id : 245u32 , pattern : "NOTHING" , len : 7usize } ,
      ScanPattern { id : 146u32 , pattern : "NATURAL" , len : 7usize } ,
      ScanPattern { id : 76u32 , pattern : "NOTNULL" , len : 7usize } ,
      ScanPattern { id : 107u32 , pattern : "NULLS" , len : 5usize } ,
      ScanPattern { id : 158u32 , pattern : "NULL" , len : 4usize } ,
      ScanPattern { id : 35u32 , pattern : "NOT" , len : 3usize } ,
      ScanPattern { id : 91u32 , pattern : "NO" , len : 2usize } ,
    ],
    'o' => &[
      ScanPattern { id : 118u32 , pattern : "OTHERS" , len : 6usize } ,
      ScanPattern { id : 94u32 , pattern : "OFFSET" , len : 6usize } ,
      ScanPattern { id : 230u32 , pattern : "ORDER" , len : 5usize } ,
      ScanPattern { id : 146u32 , pattern : "OUTER" , len : 5usize } ,
      ScanPattern { id : 306u32 , pattern : "OVER" , len : 4usize } ,
      ScanPattern { id : 140u32 , pattern : "ON" , len : 2usize } ,
      ScanPattern { id : 67u32 , pattern : "OR" , len : 2usize } ,
      ScanPattern { id : 93u32 , pattern : "OF" , len : 2usize } ,
    ],
    'p' => &[
      ScanPattern { id : 112u32 , pattern : "PARTITION" , len : 9usize } ,
      ScanPattern { id : 113u32 , pattern : "PRECEDING" , len : 9usize } ,
      ScanPattern { id : 160u32 , pattern : "PRIMARY" , len : 7usize } ,
      ScanPattern { id : 95u32 , pattern : "PRAGMA" , len : 6usize } ,
      ScanPattern { id : 9u32 , pattern : "PLAN" , len : 4usize } ,
    ],
    'q' => &[
      ScanPattern { id : 8u32 , pattern : "QUERY" , len : 5usize } ,
    ],
    'r' => &[
      ScanPattern { id : 165u32 , pattern : "REFERENCES" , len : 10usize } ,
      ScanPattern { id : 238u32 , pattern : "RETURNING" , len : 9usize } ,
      ScanPattern { id : 97u32 , pattern : "RECURSIVE" , len : 9usize } ,
      ScanPattern { id : 21u32 , pattern : "ROLLBACK" , len : 8usize } ,
      ScanPattern { id : 99u32 , pattern : "RESTRICT" , len : 8usize } ,
      ScanPattern { id : 123u32 , pattern : "REINDEX" , len : 7usize } ,
      ScanPattern { id : 24u32 , pattern : "RELEASE" , len : 7usize } ,
      ScanPattern { id : 98u32 , pattern : "REPLACE" , len : 7usize } ,
      ScanPattern { id : 124u32 , pattern : "RENAME" , len : 6usize } ,
      ScanPattern { id : 114u32 , pattern : "RANGE" , len : 5usize } ,
      ScanPattern { id : 72u32 , pattern : "REGEXP" , len : 6usize } ,
      ScanPattern { id : 146u32 , pattern : "RIGHT" , len : 5usize } ,
      ScanPattern { id : 96u32 , pattern : "RAISE" , len : 5usize } ,
      ScanPattern { id : 101u32 , pattern : "ROWS" , len : 4usize } ,
      ScanPattern { id : 100u32 , pattern : "ROW" , len : 3usize } ,
    ],
    's' => &[
      ScanPattern { id : 23u32 , pattern : "SAVEPOINT" , len : 9usize } ,
      ScanPattern { id : 200u32 , pattern : "SELECT" , len : 6usize } ,
      ScanPattern { id : 176u32 , pattern : "SET" , len : 3usize } ,
    ],
    't' => &[
      ScanPattern { id : 14u32 , pattern : "TRANSACTION" , len : 11usize } ,
      ScanPattern { id : 102u32 , pattern : "TRIGGER" , len : 7usize } ,
      ScanPattern { id : 30u32 , pattern : "TABLE" , len : 5usize } ,
      ScanPattern { id : 119u32 , pattern : "TIES" , len : 4usize } ,
      ScanPattern { id : 260u32 , pattern : "THEN" , len : 4usize } ,
      ScanPattern { id : 311u32 , pattern : "TRUE" , len : 4usize } ,
      ScanPattern { id : 37u32 , pattern : "TEMP" , len : 4usize } ,
      ScanPattern { id : 25u32 , pattern : "TO" , len : 2usize } ,
    ],
    'u' => &[
      ScanPattern { id : 115u32 , pattern : "UNBOUNDED" , len : 9usize } ,
      ScanPattern { id : 163u32 , pattern : "UNIQUE" , len : 6usize } ,
      ScanPattern { id : 175u32 , pattern : "UPDATE" , len : 6usize } ,
      ScanPattern { id : 196u32 , pattern : "UNION" , len : 5usize } ,
      ScanPattern { id : 227u32 , pattern : "USING" , len : 5usize } ,
    ],
    'v' => &[
      ScanPattern { id : 105u32 , pattern : "VIRTUAL" , len : 7usize } ,
      ScanPattern { id : 103u32 , pattern : "VACUUM" , len : 6usize } ,
      ScanPattern { id : 211u32 , pattern : "VALUES" , len : 6usize } ,
      ScanPattern { id : 104u32 , pattern : "VIEW" , len : 4usize } ,
    ],
    'w' => &[
      ScanPattern { id : 47u32 , pattern : "WITHOUT" , len : 7usize } ,
      ScanPattern { id : 305u32 , pattern : "WINDOW" , len : 6usize } ,
      ScanPattern { id : 237u32 , pattern : "WHERE" , len : 5usize } ,
      ScanPattern { id : 106u32 , pattern : "WITH" , len : 4usize } ,
      ScanPattern { id : 259u32 , pattern : "WHEN" , len : 4usize } ,
    ],
    '|' => &[
      ScanPattern { id : 136u32 , pattern : "||" , len : 2usize } ,
      ScanPattern { id : 128u32 , pattern : "|" , len : 1usize } ,
    ],
    '~' => &[
      ScanPattern { id : 139u32 , pattern : "~" , len : 1usize } ,
    ],
  };
  pub static REGEX_SCAN_RULE: &[ScanPattern] = &[
      ScanPattern { id : 247u32 , pattern : "(x|X)'.*?'" , len : 10usize } ,
      ScanPattern { id : 325u32 , pattern : "(?s)/\\*.*?\\*/" , len : 13usize } ,
      ScanPattern { id : 325u32 , pattern : "--.*" , len : 4usize } ,
      ScanPattern { id : 246u32 , pattern : "((\\d+(_\\d+)*)?[.]\\d+(_\\d+)*(e[+-]?\\d+(_\\d+)*)?)|(\\d+(_\\d+)*[.](e[+-]?\\d+(_\\d+)*)?)" , len : 82usize } ,
      ScanPattern { id : 84u32 , pattern : "\".*?\"" , len : 5usize } ,
      ScanPattern { id : 84u32 , pattern : "[a-zA-Z_][0-9a-zA-Z_]*" , len : 22usize } ,
      ScanPattern { id : 142u32 , pattern : "\".*?\"" , len : 5usize } ,
      ScanPattern { id : 142u32 , pattern : "[a-zA-Z_][0-9a-zA-Z_]*" , len : 22usize } ,
      ScanPattern { id : 248u32 , pattern : "(\\d+(_\\d+)*)" , len : 12usize } ,
      ScanPattern { id : 323u32 , pattern : "(x|X)(\\d+(_\\d+)*)" , len : 17usize } ,
      ScanPattern { id : 324u32 , pattern : "\\s+" , len : 3usize } ,
      ScanPattern { id : 144u32 , pattern : "'.*?'" , len : 5usize } ,
  ];
  pub static SUPPORT_LEADING: &[usize] = &[
      1, // (?s)/\*.*?\*/
      2, // --.*
      10, // \s+
  ];
  pub static SUPPORT_TRAILING: &[usize] = &[
      10, // \s+
  ];
  pub static SUPPORT_MAIN: &[usize] = &[
      0, // (x|X)'.*?'
      3, // ((\d+(_\d+)*)?[.]\d+(_\d+)*(e[+-]?\d+(_\d+)*)?)|(\d+(_\d+)*[.](e[+-]?\d+(_\d+)*)?)
      4, // ".*?"
      5, // [a-zA-Z_][0-9a-zA-Z_]*
      6, // ".*?"
      7, // [a-zA-Z_][0-9a-zA-Z_]*
      8, // (\d+(_\d+)*)
      9, // (x|X)(\d+(_\d+)*)
      11, // '.*?'
  ];
  pub static ALTERNATIVE_SYMBOL_TABLE: phf::Map<u32, &[u32]> = phf::phf_map!{
    133u32 => &[
      133, // STAR
      320, // ASTERISK
    ],
  };
}
