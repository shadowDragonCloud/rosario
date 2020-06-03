use crate::utils::get_selector;
use anyhow::anyhow;
use anyhow::Context;
use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use rand::seq::SliceRandom;
use scraper::element_ref::ElementRef;
use scraper::html::Select;
use scraper::Html;
use scraper::Selector;
use std::env;
use std::fmt;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path;
use std::sync::RwLock;

lazy_static! {
    static ref PROXIES: RwLock<Vec<ProxyInfo>> = RwLock::new(Vec::new());
}

const PROXY_FILE: &str = "proxy";
const PROXY_FILE_OLD: &str = "proxy.old";

fn store_proxies(proxy_infos: &[ProxyInfo]) -> anyhow::Result<()> {
    debug!("begin store proxies");
    let current_dir = env::current_dir()?;
    if path::Path::new(&current_dir.join(PROXY_FILE)).is_file() {
        debug!("prxoy file found, backup it");
        fs::rename(
            current_dir.join(PROXY_FILE),
            current_dir.join(PROXY_FILE_OLD),
        )?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(PROXY_FILE)?;
    for proxy_info in proxy_infos {
        file.write_all(format!("{} {}\n", proxy_info.ip, proxy_info.port).as_bytes())?;
    }

    debug!("store proxies success");
    Ok(())
}

fn load_proxies() -> anyhow::Result<Vec<ProxyInfo>> {
    debug!("begin load proxies");
    let mut file = fs::File::open(PROXY_FILE)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
    for line in content.lines() {
        let mut proxy_info = ProxyInfo::default();
        let blocks: Vec<_> = line.split(' ').collect();
        if blocks.len() != 2 {
            warn!(
                "load proxies error, blocks count is not 2, line= {:?}",
                line
            );
            continue;
        }
        proxy_info.ip = blocks[0].to_owned();
        proxy_info.port = blocks[1].to_owned();
        proxy_infos.push(proxy_info);
    }

    debug!("load proxies success");
    Ok(proxy_infos)
}

pub(crate) fn init() -> anyhow::Result<()> {
    let proxy_infos = load_proxies()?;

    debug!("loaded proxy infos:");
    for proxy_info in proxy_infos.clone() {
        debug!("{}", proxy_info);
    }

    let mut global_proxies = PROXIES.write().expect("get PROXIES write lock error");
    *global_proxies = proxy_infos;

    Ok(())
}

pub(crate) fn get_and_store_valid_proxies() -> anyhow::Result<()> {
    let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
    // kuaidili proxy
    info!("begin parse kuaidaili proxies");
    let kuaidaili_targets: Vec<i32> = (1..=10).collect();
    let kuaidaili_proxy_infos = parse_kuaidaili_proxy_info(kuaidaili_targets.as_slice());
    proxy_infos.extend(kuaidaili_proxy_infos);

    // xicidaili proxy
    info!("begin parse xicidaili proxies");
    let xicidaili_targets: Vec<i32> = (1..=5).collect();
    let xicidaili_proxy_infos = parse_xicidaili_proxy_info(xicidaili_targets.as_slice());
    proxy_infos.extend(xicidaili_proxy_infos);
    info!("parse proxies end, bengin test proxies");

    let valid_proxy_infos: Vec<ProxyInfo> = proxy_infos
        .iter()
        .filter_map(|proxy_info| match test_proxy_info(proxy_info) {
            Ok(true) => {
                debug!("test proxy info {:?}, result true", proxy_info);
                Some((*proxy_info).clone())
            }
            Ok(false) => {
                debug!("test proxy info {:?}, result false", proxy_info);
                None
            }
            Err(e) => {
                debug!("test proxy info {:?}, result false, e= {:?}", proxy_info, e);
                None
            }
        })
        .collect();

    info!("valid proxy infos:");
    for valid_proxy_info in valid_proxy_infos.clone() {
        info!("{}", valid_proxy_info);
    }
    store_proxies(valid_proxy_infos.as_slice())?;

    Ok(())
}

pub(crate) fn get_proxy_to_use() -> anyhow::Result<ProxyInfo> {
    PROXIES
        .read()
        .expect("failed to get PROXIES read lock")
        .choose(&mut rand::thread_rng())
        .map(|v| (*v).clone())
        .ok_or_else(|| anyhow!("failed to get random proxy"))
}

fn test_proxy_info(proxy_info: &ProxyInfo) -> anyhow::Result<bool> {
    const TEST_URL: &str = "https://book.douban.com";

    let proxy_str = format!("http://{}:{}", proxy_info.ip, proxy_info.port);
    let proxy = reqwest::Proxy::http(proxy_str.as_str())?;
    let client = reqwest::blocking::Client::builder()
        .proxy(proxy)
        .timeout(Some(std::time::Duration::from_secs(3)))
        .build()?;
    let resp = client.get(TEST_URL).send()?;

    let status = resp.status();
    let text = resp.text()?;
    trace!(
        "test prox info result, proxy_info: {:?}, response status: {:?}, response text: {:?}",
        proxy_info,
        status,
        text
    );

    if status != 200 {
        return Ok(false);
    }

    if text.contains("sec.douban.com") {
        return Ok(false);
    }

    Ok(true)
}

fn parse_kuaidaili_proxy_info(targets: &[i32]) -> Vec<ProxyInfo> {
    const BASE_URL: &str = "https://www.kuaidaili.com/free/inha/";

    let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
    for target in targets {
        let url = format!("{}{}/", BASE_URL, target.to_string());
        debug!("a new page will be parsed, url= {:?}", url);
        match parse_kuaidaili_proxy_info_from_page(url.as_str()) {
            Ok(proxy_infos_) => proxy_infos.extend(proxy_infos_),
            Err(e) => warn!("{:?}", e),
        }
    }

    proxy_infos
}

fn parse_kuaidaili_proxy_info_from_page(url: &str) -> anyhow::Result<Vec<ProxyInfo>> {
    let client = reqwest::blocking::Client::builder()
        .default_headers(crate::fetch::get_default_headers()?)
        .build()?;
    let resp = client.get(url).send()?;
    if resp.status() != 200 {
        return Err(anyhow!(
            "failed to parse proxy info, response is not 200, url= {:?}",
            url
        ));
    }

    let text = resp
        .text()
        .with_context(|| format!("failed to parse proxy info, get text error, url= {:?}", url))?;

    let document = Html::parse_document(text.as_str());
    let tbody_selector = get_selector("tbody")?;
    match document.select(&tbody_selector).next() {
        Some(tbody) => {
            let tr_selector = get_selector("tr")?;
            let tr_iter = tbody.select(&tr_selector);
            let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
            for tr in tr_iter {
                match parse_kuaidaili_proxy_info_from_tr(tr) {
                    Ok(proxy_info) => {
                        debug!("parse proxy info success, proxy_info= {:?}", proxy_info);
                        proxy_infos.push(proxy_info)
                    }
                    Err(e) => warn!("failed to parse proxy info, e= {:?}, url= {:?}", e, url),
                }
            }

            Ok(proxy_infos)
        }
        None => Err(anyhow!(
            "failed to parse proxy info, tbody not found, url= {:?}",
            url
        )),
    }
}

fn parse_kuaidaili_proxy_info_from_tr(tr: ElementRef) -> anyhow::Result<ProxyInfo> {
    let td_ip_selector = get_selector(r#"td[data-title="IP"]"#)?;
    let td_port_selector = get_selector(r#"td[data-title="PORT"]"#)?;
    let td_scheme_selector = get_selector(r#"td[data-title="类型"]"#)?;
    let td_last_verified_selector = get_selector(r#"td[data-title="最后验证时间"]"#)?;
    let td_anonymous_selector = get_selector(r#"td[data-title="匿名度"]"#)?;
    let td_position_selector = get_selector(r#"td[data-title="位置"]"#)?;

    let mut proxy_info = ProxyInfo::default();
    proxy_info.ip = parse_kuaidaili_proxy_info_from_tr_inner(tr, td_ip_selector)?;
    proxy_info.port = parse_kuaidaili_proxy_info_from_tr_inner(tr, td_port_selector)?;
    proxy_info.scheme = parse_kuaidaili_proxy_info_from_tr_inner(tr, td_scheme_selector)?;
    proxy_info.last_verified =
        parse_kuaidaili_proxy_info_from_tr_inner(tr, td_last_verified_selector)?;
    proxy_info.anonymous = parse_kuaidaili_proxy_info_from_tr_inner(tr, td_anonymous_selector)?;
    proxy_info.position = parse_kuaidaili_proxy_info_from_tr_inner(tr, td_position_selector)?;

    Ok(proxy_info)
}

fn parse_kuaidaili_proxy_info_from_tr_inner(
    tr: ElementRef,
    sel: Selector,
) -> anyhow::Result<String> {
    match tr.select(&sel).next() {
        Some(td) => {
            let texts: Vec<&str> = td.text().filter(|t| !t.trim().is_empty()).collect();
            if texts.is_empty() {
                Ok(String::new())
            } else {
                Ok(texts[0].to_owned())
            }
        }
        None => Err(anyhow!(
            "failed to parse proxy info from tr, td is None, selector= {:?}",
            sel
        )),
    }
}

fn parse_xicidaili_proxy_info(targets: &[i32]) -> Vec<ProxyInfo> {
    const BASE_URL: &str = "https://www.xicidaili.com/nn/";

    let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
    for target in targets {
        let url = format!("{}{}", BASE_URL, target.to_string());
        debug!("a new page will be parsed, url= {:?}", url);
        match parse_xicidaili_proxy_info_from_page(url.as_str()) {
            Ok(proxy_infos_) => proxy_infos.extend(proxy_infos_),
            Err(e) => warn!("{:?}", e),
        }
    }

    proxy_infos
}

fn parse_xicidaili_proxy_info_from_page(url: &str) -> anyhow::Result<Vec<ProxyInfo>> {
    let client = reqwest::blocking::Client::builder()
        .default_headers(crate::fetch::get_default_headers()?)
        .build()?;
    let resp = client.get(url).send()?;
    if resp.status() != 200 {
        return Err(anyhow!(
            "failed to parse proxy info, response is not 200, url= {:?}",
            url
        ));
    }

    let text = resp
        .text()
        .with_context(|| format!("failed to parse proxy info, get text error, url= {:?}", url))?;

    let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
    let document = Html::parse_document(text.as_str());
    let tr_odd_selector = get_selector(r#"tr[class="odd"]"#)?;
    let tr_even_selector = get_selector(r#"tr[class=""]"#)?;

    let proxy_odd_infos =
        parse_xicidaili_proxy_info_from_tr_iter(document.select(&tr_odd_selector))?;
    proxy_infos.extend(proxy_odd_infos);
    let proxy_even_infos =
        parse_xicidaili_proxy_info_from_tr_iter(document.select(&tr_even_selector))?;
    proxy_infos.extend(proxy_even_infos);

    Ok(proxy_infos)
}

fn parse_xicidaili_proxy_info_from_tr_iter(tr_iter: Select) -> anyhow::Result<Vec<ProxyInfo>> {
    let td_selector = get_selector("td")?;
    let mut proxy_infos: Vec<ProxyInfo> = Vec::new();
    for tr in tr_iter {
        let mut td_iter = tr.select(&td_selector);
        // skip country
        td_iter.next();
        let mut proxy_info = ProxyInfo::default();
        // ip
        match td_iter.next() {
            Some(td) => match parse_xicidaili_proxy_info_from_td(td) {
                Ok(ip) => proxy_info.ip = ip,
                Err(e) => {
                    warn!("{:?}", e);
                    continue;
                }
            },
            None => {
                warn!("parse_xicidaili_proxy_info_from_tr_iter failed, ip td is empty");
                continue;
            }
        }
        // port
        match td_iter.next() {
            Some(td) => match parse_xicidaili_proxy_info_from_td(td) {
                Ok(port) => proxy_info.port = port,
                Err(e) => {
                    warn!("{:?}", e);
                    continue;
                }
            },
            None => {
                warn!("parse_xicidaili_proxy_info_from_tr_iter failed, port td is empty");
                continue;
            }
        }
        debug!("parse proxy info success, proxy_info= {:?}", proxy_info);
        proxy_infos.push(proxy_info);
    }

    Ok(proxy_infos)
}

fn parse_xicidaili_proxy_info_from_td(td: ElementRef) -> anyhow::Result<String> {
    let texts: Vec<_> = td
        .text()
        .filter_map(|v| {
            let v = v.trim();
            if v.is_empty() {
                None
            } else {
                Some(v)
            }
        })
        .collect();
    if texts.is_empty() {
        Err(anyhow!(
            "parse_xicidaili_proxy_info_from_td failed, texts is empty"
        ))
    } else {
        Ok(texts[0].to_owned())
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct ProxyInfo {
    pub(crate) ip: String,
    pub(crate) port: String,
    pub(crate) scheme: String,
    pub(crate) last_verified: String,
    pub(crate) anonymous: String,
    pub(crate) position: String,
}

impl fmt::Display for ProxyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ip: {}, port: {}, scheme: {}, last_verified: {}, anonymous: {}, position: {}",
            self.ip, self.port, self.scheme, self.last_verified, self.anonymous, self.position
        )
    }
}
