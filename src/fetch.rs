use anyhow::Context;
use lazy_static::lazy_static;
use log::{debug, trace};
use rand::Rng;
use reqwest::blocking::Client;
use reqwest::header;
use std::sync::RwLock;
use std::time;

lazy_static! {
    static ref LAST_FETCH_TIME: RwLock<time::Instant> = RwLock::new(time::Instant::now());
}

fn set_last_fetch_time() {
    let mut last_fetch_time = LAST_FETCH_TIME
        .write()
        .expect("failed to get LAST_FETCH_TIME write lock");
    *last_fetch_time = time::Instant::now();
}

fn sleep_if_fetch_too_fast() {
    // the duration from last fetch is randomly generated
    let expect_duration: u128 = rand::thread_rng().gen_range(2000, 5000);
    let actual_duratioin = LAST_FETCH_TIME
        .read()
        .expect("failed to get LAST_FETCH_TIME read lock")
        .elapsed()
        .as_millis();

    // if actual time is large, fetch directly
    if expect_duration <= actual_duratioin {
        return;
    }

    debug!(
        "fetch too fast, sleep time_ms= {:?}",
        expect_duration - actual_duratioin
    );
    std::thread::sleep(time::Duration::from_millis(
        (expect_duration - actual_duratioin) as u64,
    ));
}

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
    // control fetch speed
    sleep_if_fetch_too_fast();
    set_last_fetch_time();

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
