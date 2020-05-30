use std::fs;
use std::path;
use anyhow::Context;

const STORE_TARGET_DIR: &str = "books/";

pub(crate) fn init() -> anyhow::Result<()> {
    fs::create_dir_all(STORE_TARGET_DIR)?;
    Ok(())
}

pub(crate) fn store(book_url: &str, book: crate::book::Book) -> anyhow::Result<()> {
    let book_id = crate::utils::parse_book_id(book_url);
    let file_name = format!("{}_{}", book.title, book_id);
    let file_name_for_err = file_name.clone();
    let path = path::Path::new(STORE_TARGET_DIR).join(file_name);
    fs::write(path, format!("{}", book)).with_context(|| format!("store book to file error, book_url= {:?}, file_name= {:?}", book_url, file_name_for_err))?;
    Ok(())
}
