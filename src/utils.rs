use log::debug;
use scraper::Selector;
use scraper::node::Node;
use scraper::element_ref::ElementRef;
use ego_tree::NodeRef;
use std::iter::Iterator;
use anyhow::{Context, anyhow};

pub(crate) fn parse_book_id(book_page_url: &str) -> String {
    let url_segments = book_page_url.rsplit('/').filter_map(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }).collect::<Vec<&str>>();

    if url_segments.is_empty() {
        String::new()
    } else {
        url_segments[0].to_owned()
    }
}

pub(crate) fn get_page(url: &str) -> anyhow::Result<String> {
    let resp = reqwest::blocking::get(url).with_context(|| format!("failed to get page, url= {:?}", url))?;
    debug!("response status: {:?}, url= {:?}", resp.status(), url);
    resp.text().with_context(|| format!("faild to get resp text, url= {:?}", url))
}

pub(crate) fn get_selector(selector_str: &str) -> anyhow::Result<Selector> {
    Selector::parse(selector_str).map_err(|e| anyhow!("{:?}",e )).with_context(|| format!("get selector error, selector_str= {:?}", selector_str))
}

pub(crate) fn node_ref_text(node_ref: NodeRef<Node>) -> Vec<String> {
    if let Some(element_ref) = ElementRef::wrap(node_ref) {
        element_ref.text().map(|s| s.to_owned()).collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}
