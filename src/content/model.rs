use chrono::NaiveDate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub slug: String,
    pub title: String,
    pub kind: Option<String>,
    pub last_edit: Option<NaiveDate>,
    pub html: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub publish_date: NaiveDate,
    pub last_edit: Option<NaiveDate>,
    pub kind: Option<String>,
    pub draft: bool,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub html: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagSummary {
    pub name: String,
    pub post_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoModule {
    pub repository_url: String,
    pub module_path: String,
    pub name: String,
    pub description: String,
    pub license: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branding {
    pub word: String,
    pub slogan: String,
}
