
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
