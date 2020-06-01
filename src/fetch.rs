use anyhow::anyhow;
use anyhow::Context;
use lazy_static::lazy_static;
use log::{debug, trace};
use reqwest::blocking::Client;
use reqwest::header;
use std::sync::RwLock;

lazy_static! {
    static ref CLIENT: RwLock<Client> = RwLock::new(Client::new());
}

fn get_deafult_headers() -> anyhow::Result<header::HeaderMap> {
    const USER_AGENT_VALUE: &str = r#"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/81.0.4044.138 Safari/537.36"#;
    const HOST_VALUE: &str = r#"book.douban.com"#;
    const CONNECTION_VALUE: &str = r#"keep-alive"#;
    const ACCEPT_VALUE: &str = r#"text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9"#;
    const ACCEPT_ENCODING_VALUE: &str = r#"gzip deflate"#;
    const ACCEPT_LANGUAGE_VALUE: &str = r#"zh-CN,zh;q=0.9"#;

    let mut headers = header::HeaderMap::new();

    headers.insert(header::USER_AGENT, USER_AGENT_VALUE.parse()?);
    headers.insert(header::HOST, HOST_VALUE.parse()?);
    headers.insert(header::CONNECTION, CONNECTION_VALUE.parse()?);
    headers.insert(header::ACCEPT, ACCEPT_VALUE.parse()?);
    headers.insert(header::ACCEPT_ENCODING, ACCEPT_ENCODING_VALUE.parse()?);
    headers.insert(header::ACCEPT_LANGUAGE, ACCEPT_LANGUAGE_VALUE.parse()?);

    Ok(headers)
}

pub(crate) fn init() -> anyhow::Result<()> {
    let mut global_client = CLIENT.write().expect("get CLIENT write lock error");
    *global_client = Client::builder()
        .default_headers(get_deafult_headers()?)
        .gzip(true)
        .build()?;

    Ok(())
}

pub(crate) fn get_page(url: &str, referrer: &str) -> anyhow::Result<String> {
    let resp = CLIENT
        .read()
        .map_err(|e| anyhow!("failed to get page, e= {:?} url= {:?}", e, url))?
        .get(url)
        .header(header::REFERER, referrer)
        .send()
        .with_context(|| format!("failed to get page, url= {:?}", url))?;
    debug!("response status: {:?}, url= {:?}", resp.status(), url);
    let text = resp
        .text()
        .with_context(|| format!("faild to get resp text, url= {:?}", url))?;
    trace!("response text: {:?}", text);

    Ok(text)
}
