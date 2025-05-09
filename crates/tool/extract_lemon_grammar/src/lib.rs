mod keyword_check;


#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use crate::keyword_check::sqlite3_keyword_check;

    #[test]
    fn test_keyword_ckeck() -> Result<(), anyhow::Error> {
        unsafe {
            let needle = "SELECT".to_lowercase();
            assert_eq!(1, sqlite3_keyword_check(CString::new(needle.as_str())?.as_ptr(), needle.len() as i32));

            let needle = "selcollist";
            assert_eq!(0, sqlite3_keyword_check(CString::new(needle)?.as_ptr(), needle.len() as i32));
        }
        Ok(())
    }
}
