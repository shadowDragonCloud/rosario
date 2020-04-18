use scraper::{Html, Selector};

fn main() {
    let host = "https://book.douban.com";
    let tags_href = match get_and_parse_root_page() {
        Ok(tags_href) => tags_href,
        Err(e) => {
            println!("{:?}", e);
            return;
        }
    };

    for tag_href in tags_href {
        let tag_url = format!("{}{}", host, tag_href);
        let _books_url = match get_and_parse_tag_page(tag_url.as_str()) {
            Ok(books_url) => books_url,
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        };
        break;
    }
}

fn get_and_parse_root_page() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let root_url = "https://book.douban.com/tag/";
    let resp = reqwest::blocking::get(root_url)?;
    println!("response status: {:?}", resp.status());
    let document = Html::parse_document(resp.text()?.as_str());
    let table_selector = Selector::parse(r#"table[class="tagCol"]"#).unwrap();
    let a_selector = Selector::parse("a").unwrap();

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

fn get_and_parse_tag_page(tag_page_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(tag_page_url)?;
    println!("response status: {:?}", resp.status());
    let document = Html::parse_document(resp.text()?.as_str());
    let li_selector = Selector::parse(r#"li[class="subject-item"]"#).unwrap();
    let h2_selector = Selector::parse("h2").unwrap();
    let a_selector = Selector::parse("a").unwrap();

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
                books_url.push(url.clone());
                println!("title: {:?}, book_url: {:?}", title, url);
            }
        }
    }
    Ok(books_url)
}
