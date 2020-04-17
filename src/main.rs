use scraper::{Html, Selector};

fn main() {
    let url = "https://book.douban.com/tag/";
    if let Err(e) = get_and_parse_page(url) {
        println!("{:?}", e);
    }
}

fn get_and_parse_page(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(url)?;
    println!("response status: {:?}", resp.status());
    let document = Html::parse_document(resp.text()?.as_str());
    let table_selector = Selector::parse(r#"table[class="tagCol"]"#).unwrap();
    let a_selector = Selector::parse("a").unwrap();

    for table in document.select(&table_selector) {
        for a in table.select(&a_selector) {
            println!("{:?}", a.value().attr("href"));
        }
    }
    Ok(())
}
