use std::fmt;

#[derive(Default)]
pub struct Score {
    pub score: f32,
    pub score_num: i32,
    pub five_star_pct: f32,
    pub four_star_pct: f32,
    pub three_star_pct: f32,
    pub two_star_pct: f32,
    pub one_star_pct: f32
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "评分:\n豆瓣评分: {}", self.score)?;
        writeln!(f, "评价人数: {}", self.score_num)?;
        writeln!(f, "5星: {}", self.five_star_pct)?;
        writeln!(f, "4星: {}", self.four_star_pct)?;
        writeln!(f, "3星: {}", self.three_star_pct)?;
        writeln!(f, "2星: {}", self.two_star_pct)?;
        write!(f, "1星: {}", self.one_star_pct)
    }
}

#[derive(Default)]
pub struct Book {
    pub title: String,
    pub location: String,
    pub origin_title: String,
    pub author: Vec<String>,
    pub translator: Vec<String>,
    pub press: String,
    pub producer: String,
    pub publication_year: String,
    pub page_num: String,
    pub price: String,
    pub binding: String,
    pub series: String,
    pub isbn: String,
    pub score: Score,

    pub content_intro: String,
    pub author_intro: String,
    pub directory: String,
}

impl fmt::Display for Book {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "书名: {}", self.title)?;
        writeln!(f, "URL: {}", self.location)?;
        writeln!(f, "原作名: {}", self.origin_title)?;
        writeln!(f, "作者: {}", vec2comma_seperated_string(self.author.as_slice()))?;
        writeln!(f, "译者: {}", vec2comma_seperated_string(self.translator.as_slice()))?;
        writeln!(f, "出版社: {}", self.press)?;
        writeln!(f, "出品方: {}", self.producer)?;
        writeln!(f, "出版年: {}", self.publication_year)?;
        writeln!(f, "页数: {}", self.page_num)?;
        writeln!(f, "定价: {}", self.price)?;
        writeln!(f, "装帧: {}", self.binding)?;
        writeln!(f, "丛书: {}", self.series)?;
        writeln!(f, "isbn: {}", self.isbn)?;
        writeln!(f, "\n{}\n", self.score)?;
        writeln!(f, "内容简介:\n{}\n", self.content_intro)?;
        writeln!(f, "作者简介:\n{}\n", self.author_intro)?;
        write!(f, "目录:\n{}", self.directory)
    }
}

fn vec2comma_seperated_string(v: &[String]) -> String {
    let mut res = String::new();
    let len = v.len();
    if len == 0 {
        return res;
    }

    for i in 0..len-1 {
        res.push_str(v[i].as_str());
        res.push_str(", ");
    }
    res.push_str(v[len-1].as_str());
    res
}
