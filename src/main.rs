use crate::parser::{
    book_page::get_and_parse_book_page,
    root_page::get_and_parse_root_page,
    tag_page::{get_and_parse_tag_page, get_max_tag_page_count},
};
use log::{debug, error, info, warn};
use std::env;

mod book;
mod fetch;
mod logs;
mod parser;
mod proxy;
mod store;
mod utils;

#[allow(clippy::cognitive_complexity)]
fn main() {
    // init log
    if let Err(e) = crate::logs::init() {
        println!("init log failed, e= {:?}", e);
        return;
    }

    // get valid proxy ip pool
    // use cargo run proxy first
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 && args[1] == "proxy" {
        info!("get and parse proxy pool");
        if let Err(e) = crate::proxy::get_and_store_valid_proxies() {
            warn!("failed to get and store valid proxies, e= {:?}", e);
        }

        return;
    }

    // load valid proxy parsed by last step
    if let Err(e) = crate::proxy::init() {
        error!("init proxy failed, e= {:?}", e);
        return;
    }

    // init store
    if let Err(e) = crate::store::init() {
        error!("init store failed, e= {:?}", e);
        return;
    }

    // parse root page
    let tags_href = match get_and_parse_root_page() {
        Ok(tags_href) => tags_href,
        Err(e) => {
            error!("{:?}", e);
            return;
        }
    };
    info!("parse root page success");
    debug!("tags_href= {:?}", tags_href);

    const HOST: &str = "https://book.douban.com";
    const ROOT_URL: &str = "https://book.douban.com/tag/";
    for tag_href in tags_href {
        // parse tag page, get max tag page count
        let tag_url = format!("{}{}", HOST, tag_href);
        let max_tag_page_count = match get_max_tag_page_count(tag_url.as_str(), ROOT_URL) {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    "failed to get max tag page count, ignore this tag, e= {:?}",
                    e
                );
                continue;
            }
        };
        if max_tag_page_count == 0 {
            warn!(
                "max tag page count is zero, ignore this tag, tag_page_url= {:?}",
                tag_url
            );
            continue;
        }
        info!(
            "get max tag page count success, count= {:?}, tag_page_url= {:?}",
            max_tag_page_count, tag_url
        );

        // trace all tag pages of a tag
        const COUNT_PER_PAGE: i32 = 20;
        for idx in 0..max_tag_page_count {
            // parse tag page, get book urls
            let tag_page_url = format!("{}?start={}&type=T", tag_url, idx * COUNT_PER_PAGE);
            let referrer = if idx == 0 {
                tag_url.clone()
            } else {
                format!("{}?start={}&type=T", tag_url, (idx - 1) * COUNT_PER_PAGE)
            };
            let books_url = match get_and_parse_tag_page(tag_page_url.as_str(), referrer.as_str()) {
                Ok(books_url) => books_url,
                Err(e) => {
                    warn!("{:?}", e);
                    continue;
                }
            };
            info!("parse tag page suceess, url= {:?}", tag_page_url);

            // parse book page, get book info
            for book_url in books_url {
                let book = match get_and_parse_book_page(book_url.as_str(), tag_page_url.as_str()) {
                    Ok(book) => book,
                    Err(e) => {
                        warn!("parse book page failed, e= {:?}, url= {:?}", e, book_url);
                        continue;
                    }
                };
                let book_title = book.title.clone();
                info!(
                    "parse book success, title= {:?}, url= {:?}",
                    book_title, book_url
                );
                if let Err(e) = crate::store::store(book_url.as_str(), book) {
                    warn!("store book page failed, e= {:?}, url= {:?}", e, book_url);
                }
                info!(
                    "store book success, title= {:?}, url= {:?}",
                    book_title, book_url
                );
            }

            info!(
                "store all books in this tag page success, tag_page_url= {:?}",
                tag_page_url
            );
            if idx >= 1 {
                break;
            }
        }

        break;
    }
}
