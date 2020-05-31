use crate::utils::{get_page, get_selector, node_ref_text};
use scraper::Html;
use scraper::node::Node;
use scraper::element_ref::ElementRef;
use std::iter::Iterator;
use anyhow::anyhow;
use log::{debug, warn};

pub(crate) fn get_max_tag_page_count(tag_page_url: &str) -> anyhow::Result<i32> {
    let resp_text = get_page(tag_page_url)?;
    let document = Html::parse_document(resp_text.as_str());

    let mut max_tag_page_count = 0;
    let div_paginator_selector = get_selector(r#"div[class="paginator"]"#)?;
    match document.select(&div_paginator_selector).next() {
        Some(div_paginator) => {
            let a_texts = parse_children_a_texts(div_paginator, tag_page_url);
            for text in a_texts {
                match text.parse::<i32>() {
                    Ok(v) => max_tag_page_count = v,
                    Err(e) => warn!("get max page count error, e= {:?}, tag_page_url= {:?}", e, tag_page_url)
                }
            }
            Ok(max_tag_page_count)
        }
        None => {
            Err(anyhow!("parse max tag page count error, no paginator found"))
        }
    }
}

pub(crate) fn get_and_parse_tag_page(tag_page_url: &str) -> anyhow::Result<Vec<String>> {
    let resp_text = get_page(tag_page_url)?;
    let document = Html::parse_document(resp_text.as_str());
    let li_selector = get_selector(r#"li[class="subject-item"]"#)?;
    let h2_selector = get_selector("h2")?;
    let a_selector = get_selector("a")?;

    let mut books_url = Vec::new();
    for li in document.select(&li_selector) {
        if let Some(h2) = li.select(&h2_selector).next() {
            if let Some(a) = h2.select(&a_selector).next() {
                let mut url: String = String::new();
                let mut title: String = String::new();
                if let Some(href) = a.value().attr("href") {
                    url = String::from(href);
                }
                if let Some(t) = a.value().attr("title") {
                    title = String::from(t);
                }
                debug!("parse new book url, title: {:?}, book_url: {:?}", title, url);
                if url.is_empty() {
                    warn!("href not found, tag_page_url= {:?}", tag_page_url);
                    continue;
                }
                books_url.push(url.clone());
            }
        }
    }
    Ok(books_url)
}

fn parse_children_a_texts(element_ref: ElementRef, tag_page_url: &str) -> Vec<String> {
    let mut a_texts: Vec<String> = Vec::new();
    let children = element_ref.children();
    for child in children {
        const A_NAME: &str = "a";
        match child.value() {
            Node::Element(element) if element.name() == A_NAME => {
                let texts = node_ref_text(child);
                if !texts.is_empty() {
                    a_texts.push(texts[0].trim().to_owned());
                } else {
                    warn!("get max page count error, text is empty, tag_page_url= {:?}", tag_page_url);
                }
            }
            _ => ()
        }
    }

    a_texts
}
