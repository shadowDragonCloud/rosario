use anyhow::Context;
use lazy_static::lazy_static;
use log::{debug, warn};
use std::collections::HashSet;
use std::fs;
use std::path;
use std::sync::RwLock;

lazy_static! {
    static ref STORED_BOOK_IDS: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
}

fn add_stored_book_id(book_id: &str) {
    let mut stored_book_ids = STORED_BOOK_IDS
        .write()
        .expect("failed to get STORED_BOOK_IDS write lock");
    if !stored_book_ids.insert(book_id.to_owned()) {
        warn!("duplicated book id found, book_id= {:?}", book_id);
    }
}

pub(crate) fn is_already_store(book_url: &str) -> bool {
    let book_id = crate::utils::parse_book_id(book_url);
    STORED_BOOK_IDS
        .read()
        .expect("failed to get STORED_BOOK_IDS read lock")
        .contains(&book_id)
}

const STORE_TARGET_DIR: &str = "books/";

pub(crate) fn init() -> anyhow::Result<()> {
    fs::create_dir_all(STORE_TARGET_DIR)?;
    for entry in fs::read_dir(STORE_TARGET_DIR)? {
        let entry = match entry {
            Ok(entry_) => entry_,
            Err(e) => {
                warn!("failed to get entry name, e= {:?}", e);
                continue;
            }
        };

        let name = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(e) => {
                warn!("failed to get file name, e= {:?}", e);
                continue;
            }
        };

        let name_blocks: Vec<_> = name
            .rsplit('_')
            .filter_map(|v| {
                let v = v.trim();
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            })
            .collect();

        if name_blocks.is_empty() {
            warn!("failed to get book id, name blocks is empty");
            continue;
        }

        debug!(
            "new stoed book found, name= {:?}, book_id= {:?}",
            name, name_blocks[0]
        );
        add_stored_book_id(name_blocks[0]);
    }

    Ok(())
}

pub(crate) fn store(book_url: &str, book: crate::book::Book) -> anyhow::Result<()> {
    let book_id = crate::utils::parse_book_id(book_url);
    let file_name = format!("{}_{}", book.title, book_id);
    let file_name_for_err = file_name.clone();
    let path = path::Path::new(STORE_TARGET_DIR).join(file_name);
    fs::write(path, format!("{}", book)).with_context(|| {
        format!(
            "store book to file error, book_url= {:?}, file_name= {:?}",
            book_url, file_name_for_err
        )
    })?;

    add_stored_book_id(book_id.as_str());

    Ok(())
}
