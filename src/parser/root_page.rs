use crate::utils::{get_page, get_selector};
use scraper::Html;

pub(crate) fn get_and_parse_root_page() -> anyhow::Result<Vec<String>> {
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
