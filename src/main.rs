use scraper::{Html, Selector};
use scraper::node::Node;
use scraper::element_ref::ElementRef;
use ego_tree::NodeRef;
use std::iter::Iterator;
use anyhow::{Context, anyhow};
use log::{debug, info, error, warn, trace};
use crate::book::{Book, Score};

mod book;
mod store;
mod utils;
mod logs;

fn main() {
    if let Err(e) = crate::logs::init() {
        println!("init log failed, e= {:?}", e);
        return;
    }
    
    if let Err(e) = crate::store::init() {
        error!("init store failed, e= {:?}", e);
        return;
    }

    let host = "https://book.douban.com";
    let tags_href = match get_and_parse_root_page() {
        Ok(tags_href) => tags_href,
        Err(e) => {
            error!("{:?}", e);
            return;
        }
    };
    info!("parse root page success");

    for tag_href in tags_href {
        let tag_url = format!("{}{}", host, tag_href);
        let max_tag_page_count = match get_max_tag_page_count(tag_url.as_str()) {
            Ok(v) => v,
            Err(e) => {
                warn!("failed to get max tag page count, ignore this tag, e= {:?}", e);
                continue;
            }
        };
        if max_tag_page_count == 0 {
            warn!("max tag page count is zero, ignore this tag, tag_page_url= {:?}", tag_url);
            continue;
        }
        info!("get max tag page count success, count= {:?}, tag_page_url= {:?}", max_tag_page_count, tag_url);

        const COUNT_PER_PAGE:i32 = 20;
        for idx in 0..max_tag_page_count {
            let tag_page_url = format!("{}?start={}&type=T", tag_url, idx*COUNT_PER_PAGE);
            let books_url = match get_and_parse_tag_page(tag_page_url.as_str()) {
                Ok(books_url) => books_url,
                Err(e) => {
                    warn!("{:?}", e);
                    continue;
                }
            };
            info!("parse tag page suceess, url= {:?}", tag_page_url);

            for book_url in books_url {
                let book = match get_and_parse_book_page(book_url.as_str()) {
                    Ok(book) => book,
                    Err(e) => {
                        warn!("parse book page failed, e= {:?}", e);
                        continue;
                    }
                };
                let book_title = book.title.clone();
                info!("parse book success, title= {:?}, url= {:?}", book_title, book_url);
                if let Err(e) = crate::store::store(book_url.as_str(), book) {
                    warn!("store book page failed, e= {:?}", e);
                }
                info!("store book success, title= {:?}, url= {:?}", book_title, book_url);
            }

            info!("store all books in this tag page success, tag_page_url= {:?}", tag_page_url);
            if idx >= 2 {
                break;
            }
        }
        
        break;
    }
}

fn get_page(url: &str) -> anyhow::Result<String> {
    let resp = reqwest::blocking::get(url).with_context(|| format!("failed to get page, url= {:?}", url))?;
    debug!("response status: {:?}, url= {:?}", resp.status(), url);
    resp.text().with_context(|| format!("faild to get resp text, url= {:?}", url))
}

fn get_selector(selector_str: &str) -> anyhow::Result<Selector> {
    Selector::parse(selector_str).map_err(|e| anyhow!("{:?}",e )).with_context(|| format!("get selector error, selector_str= {:?}", selector_str))
}

fn get_and_parse_root_page() -> anyhow::Result<Vec<String>> {
    let root_url = "https://book.douban.com/tag/";
    let resp_text = get_page(root_url)?;
    let document = Html::parse_document(resp_text.as_str());
    let table_selector = get_selector(r#"table[class="tagCol"]"#)?;
    let a_selector = get_selector("a")?;

    let mut tags_href = Vec::new();
    for table in document.select(&table_selector) {
        for a in table.select(&a_selector) {
            if let Some(href) = a.value().attr("href") {
                tags_href.push(String::from(href));
            }
        }
    }
    Ok(tags_href)
}

fn get_max_tag_page_count(tag_page_url: &str) -> anyhow::Result<i32> {
    let resp_text = get_page(tag_page_url)?;
    let document = Html::parse_document(resp_text.as_str());

    let mut max_tag_page_count = 0;
    let div_paginator_selector = get_selector(r#"div[class="paginator"]"#)?;
    match document.select(&div_paginator_selector).next() {
        Some(div_paginator) => {
            let children = div_paginator.children();
            for child in children {
                const A_NAME: &str = "a";
                match child.value() {
                    Node::Element(element) if element.name() == A_NAME => {
                        let texts = node_ref_text(child);
                        if !texts.is_empty() {
                            match texts[0].trim().parse::<i32>() {
                                Ok(v) => max_tag_page_count = v,
                                Err(e) => warn!("get max page count error, e= {:?}, tag_page_url= {:?}", e, tag_page_url)
                            }
                        } else {
                            warn!("get max page count error, text is empty, tag_page_url= {:?}", tag_page_url);
                        }
                    }
                    _ => ()
                }
            }
            Ok(max_tag_page_count)
        }
        None => {
            Err(anyhow!("parse max tag page count error, no paginator found"))
        }
    }
}

fn get_and_parse_tag_page(tag_page_url: &str) -> anyhow::Result<Vec<String>> {
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

fn get_and_parse_book_page(book_page_url: &str) -> anyhow::Result<Book> {
    let resp_text = get_page(book_page_url)?;
    let document = Html::parse_document(resp_text.as_str());

    let mut book: Book = Default::default();

    // location
    book.location = book_page_url.to_owned();
    // title
    let h1_selector = get_selector("h1")?;
    if let Some(h1) = document.select(&h1_selector).next() {
        let span_selector = get_selector("span")?;
        if let Some(span) = h1.select(&span_selector).next() {
            let texts = span.text().collect::<Vec<_>>();
            if !texts.is_empty() {
                book.title = texts[0].trim().to_owned();
            }
        }
    }
    // basic info
    let div_info_selector = get_selector(r#"div[id="info"]"#)?;
    if let Some(div_info) = document.select(&div_info_selector).next() {
        trace!("div_info is not none");
        let span_p1_selector = get_selector(r#"span[class="pl"]"#)?;
        let span_p1_iter = div_info.select(&span_p1_selector);
        for span_p1 in span_p1_iter {
            let text = span_p1.text().collect::<Vec<_>>();
            let info_type = if !text.is_empty() {
                clean_basic_info_type(text[0])
            } else {
                warn!("parse basic info error, span_p1 is empty, url= {:?}", book.location);
                continue;
            };

            let mut info_a_texts: Vec<String> = Vec::new();
            let mut info_text_texts: Vec<String> = Vec::new();
            let siblings = span_p1.next_siblings();
            for sibling in siblings {
                if parse_basic_info_text(sibling, &mut info_a_texts, &mut info_text_texts) {
                    break;
                }
            }

            fill_book_basic_info(&mut book, info_type.as_str(), info_text_texts, info_a_texts);
        }
    }

    // score
    let mut score = Score::default();
    let div_rating_wrap_selector = get_selector(r#"div[class="rating_wrap clearbox"]"#)?;
    if let Some(div_rating_wrap) = document.select(&div_rating_wrap_selector).next() {
        // rating_num
        let strong_rating_num_selector = get_selector(r#"strong[class="ll rating_num "]"#)?;
        match div_rating_wrap.select(&strong_rating_num_selector).next() {
            Some(strong_rating_num) => {
                let text = strong_rating_num.text().collect::<Vec<_>>();
                if text.is_empty() {
                    warn!("parse score error, rating_num text is empty, url= {:?}", book.location);
                } else {
                    let rating_num = text[0].trim().parse::<f32>();
                    match rating_num {
                        Ok(rating_num) => score.score = rating_num,
                        Err(e) => warn!("parse score error, parse rating_num to f32 fail, e= {:?}, url= {:?}", e, book.location)
                    }
                }
            },
            None => warn!("parse score error, strong_rating_num is empty, url= {:?}", book.location)
        }

        // rating_people
        let a_rating_people_selector = get_selector(r#"a[class="rating_people"]"#)?;
        match div_rating_wrap.select(&a_rating_people_selector).next() {
            Some(a_rating_people) => {
                let text = a_rating_people.text().collect::<Vec<_>>();
                if text.is_empty() {
                    warn!("parse score error, rating_people text is empty, url= {:?}", book.location);
                } else {
                    let rating_people = text[0].trim().parse::<i32>();
                    match rating_people {
                        Ok(rating_people) => score.score_num = rating_people,
                        Err(e) => warn!("parse score error, parse rating_people to i32 fail, e= {:?}, url= {:?}", e, book.location)
                    }
                }
            },
            None => warn!("parse score error, a_rating_people is empty, url= {:?}", book.location)
        }

        // star percent
        let div_rating_self_selector = get_selector(r#"div[class="rating_self clearfix"]"#)?;
        match div_rating_wrap.select(&div_rating_self_selector).next() {
            Some(div_rating_self) => {
                let mut expect_star_value = false;
                let mut star_desc = String::new();

                let siblings = div_rating_self.next_siblings();
                for sibling in siblings {
                    match sibling.value() {
                        Node::Element(_) => {
                            let texts = node_ref_text(sibling);
                            for text in texts.iter() {
                                let value = text.trim();
                                if !value.is_empty() {
                                    if expect_star_value {
                                        let star_value = value.replace("%","").trim().parse::<f32>();
                                        match star_value {
                                            Ok(star_value) => {
                                                let fill_succ = fill_star_value(&mut score, star_value, star_desc.as_str(), book.location.as_str());
                                                if !fill_succ {
                                                    break;
                                                }
                                                star_desc = String::new();
                                                expect_star_value = false;
                                            }
                                            Err(e) => {
                                                warn!("parse score error, parse start_value to f32 fail, e= {:?}, url= {:?}", e, book.location);
                                                break;
                                            }
                                        }
                                    } else {
                                        star_desc = value.to_owned();
                                        expect_star_value = true;
                                    }
                                }
                            }
                        },
                        Node::Text(text) => {
                            trace!("parse score, Text node found, text= {:?}, url= {:?}", text.trim(), book.location);
                        }
                        _ => {
                            warn!("parse score error, star percent unexpected node, url= {:?}", book.location);
                        }
                    }
                }
            },
            None => warn!("parse score error, div_rating_self is empty, url= {:?}", book.location)
        }
    }
    book.score = score;

    // related info
    let div_related_info_selector = get_selector(r#"div[class="related_info"]"#)?;
    let div_link_report_selector = get_selector(r#"div[id="link-report"]"#)?;
    let div_intro_selector = get_selector(r#"div[class="intro"]"#)?;
    if let Some(div_related_info) = document.select(&div_related_info_selector).next() {
        let mut traced_div_intro_count = 0;
        // content intro
        if let Some(div_link_report) = div_related_info.select(&div_link_report_selector).next() {
            let div_intro_iter = div_link_report.select(&div_intro_selector);
            for div_intro in div_intro_iter {
                traced_div_intro_count += 1;
                book.content_intro = clean_related_info_text(Box::new(div_intro.text()), false);
            }
        }

        // author intro
        let div_intro_iter = div_related_info.select(&div_intro_selector);
        for div_intro in div_intro_iter {
            if traced_div_intro_count != 0 {
                traced_div_intro_count -= 1;
                continue;
            }
            book.author_intro = clean_related_info_text(Box::new(div_intro.text()), false);
        }

        // directory
        let book_id = utils::parse_book_id(book_page_url);
        if book_id.is_empty() {
            warn!("parse related info, book_id is empty, book_page_url= {:?}", book_page_url);
        }
        let div_dir_id_full_selector = get_selector(format!("div[id=\"dir_{}_full\"]", book_id).as_str())?;
        if let Some(div_dir_id_full) = div_related_info.select(&div_dir_id_full_selector).next() {
            book.directory = clean_related_info_text(Box::new(div_dir_id_full.text()), true);
        }
    }


    Ok(book)
}

fn fill_star_value(score: &mut Score, star_value: f32, star_desc: &str, location: &str) -> bool {
    match star_desc {
        "5星" => score.five_star_pct = star_value,
        "4星" => score.four_star_pct = star_value,
        "3星" => score.three_star_pct = star_value,
        "2星" => score.two_star_pct = star_value,
        "1星" => score.one_star_pct = star_value,
        _ => {
            warn!("fill start value error, unknown star_desc, star_desc= {:?}, url= {:?}", star_desc, location);
            return false
        }
    }

    true
}

fn parse_basic_info_text(node_ref: NodeRef<Node>, info_a_texts: &mut Vec<String>, info_text_texts: &mut Vec<String>) -> bool {
    trace!("parse basic info, info_a_texts= {:?}, info_text_texts= {:?}", info_a_texts, info_text_texts);
    const BR_NAME: &str = "br";
    const A_NAME: &str = "a";
    let mut should_end = false;
    match node_ref.value() {
        Node::Element(element) => {
            match element.name() {
                BR_NAME => {
                    should_end = true;
                }
                A_NAME => {
                    let texts = node_ref_text(node_ref);
                    if !texts.is_empty() {
                        let info_text = clean_basic_info_text(texts[0].as_str());
                        if !info_text.is_empty() {
                            info_a_texts.push(info_text);
                        }
                    }
                },
                _ => ()
            }
        },
        Node::Text(text) => {
            let info_text = clean_basic_info_text(format!("{:?}", text).as_str());
            if !info_text.is_empty() {
                info_text_texts.push(info_text);
            }
        },
        _ => ()
    }
    
    should_end
}

fn node_ref_text(node_ref: NodeRef<Node>) -> Vec<String> {
    if let Some(element_ref) = ElementRef::wrap(node_ref) {
        element_ref.text().map(|s| s.to_owned()).collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

fn fill_book_basic_info(book: &mut Book, info_type: &str, mut info_text_texts: Vec<String>, mut info_a_texts: Vec<String>) {
    debug!("fill basic info, info_type= {:?} info_a_texts= {:?}, info_text_texts= {:?}", info_type, info_a_texts, info_text_texts);
    let info_texts = &mut info_a_texts;
    info_texts.append(&mut info_text_texts);

    let info_texts = info_a_texts;
    if info_texts.is_empty() {
        warn!("fill basic info error, info_texts is empty, url= {:?}, info_type= {:?}", book.location, info_type);
        return;
    }

    let single_info_value = info_texts[0].clone();
    match info_type {
        "原作名" => {
            book.origin_title = single_info_value;
        }
        "作者" => {
            for text in info_texts {
                book.author.push(text.replace(" ", ""));
            }
        }
        "译者" => {
            for text in info_texts {
                book.translator.push(text.replace(" ", ""));
            }
        }
        "出版社" => {
            book.press = single_info_value;
        }
        "出品方" => {
            book.producer = single_info_value;
        }
        "出版年" => {
            book.publication_year = single_info_value;
        }
        "页数" => {
            book.page_num = single_info_value;
        }
        "定价" => {
            book.price = single_info_value;
        }
        "装帧" => {
            book.binding = single_info_value;
        }
        "丛书" => {
            book.series = single_info_value;
        }
        "isbn" | "ISBN" => {
            book.isbn = single_info_value;
        }
        _ => {
            warn!("unexpected info_type, info_type= {:?}, url= {:?}", info_type, book.location);
        }
    }
}

fn clean_basic_info_type(info_type: &str) -> String {
    const TRIM_MATCH_LIST: &[char] = &['"', ':'];
    const COLON: &str = ":";
    const EMPTY: &str = "";
    info_type.trim_matches(TRIM_MATCH_LIST).replace(COLON, EMPTY).trim().to_owned()
}

fn clean_basic_info_text(info_text: &str) -> String {
    const TRIM_MATCH_LIST: &[char] = &['"', ':', '\u{a0}'];
    const REPLACE_LSIT: &[&str] = &["&;nbsp", "/", r#"\n"#, r#"\u{a0}"#, "\n"];
    const EMPTY: &str = "";
    let mut info_text = info_text.to_owned();
    for replace_entry in REPLACE_LSIT {
        info_text = info_text.replace(replace_entry, EMPTY);
    } 
    info_text.trim_matches(TRIM_MATCH_LIST).trim().to_owned()
}

fn clean_related_info_text<'a>(texts: Box<dyn Iterator<Item = &str> + 'a>, is_dir: bool) -> String 
{
    const DIR_TRIM_MATCH_LIST: &[char] = &['\n', '\t', ' ', '(', ')', '·'];
    const DIR_IGNORE_STR: &str = "收起";
    let final_intro = String::new();
    texts.filter_map(|s| {
        let mut s = s.trim();
        if is_dir {
            s = s.trim_matches(DIR_TRIM_MATCH_LIST);
            if s == DIR_IGNORE_STR {
                return None;
            }
        }
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }).fold(final_intro, |mut final_intro, s| {
        if !final_intro.is_empty() {
            final_intro.push('\n');
        }
        final_intro.push_str(s);
        final_intro
    })
}
