use anyhow::Context;
use log::{debug, trace};
use reqwest::blocking::Client;
use reqwest::header;

pub(crate) fn get_default_headers() -> anyhow::Result<header::HeaderMap> {
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

fn get_client() -> anyhow::Result<Client> {
    let proxy_info = crate::proxy::get_proxy_to_use()?;
    let proxy_str = format!("http://{}:{}", proxy_info.ip, proxy_info.port);
    debug!("used proxy: {:?}", proxy_str);
    let proxy = reqwest::Proxy::http(proxy_str.as_str())?;
    Ok(Client::builder()
        .default_headers(get_default_headers()?)
        .proxy(proxy)
        .build()?)
}

pub(crate) fn get_page(url: &str, referrer: &str) -> anyhow::Result<String> {
    let client = get_client()?;
    let resp = client 
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
