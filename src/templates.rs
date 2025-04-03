use maud::{html, Markup, DOCTYPE};
use crate::flair::{get_tagline, get_word};

fn head() -> Markup {
    html! {
        head {
            meta http-equiv="Content-Type" content="text/html;charset=utf-8";
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            meta theme-color="#a3be8c";
            title { "put a title here" }
        }
    }
}

fn header() -> Markup {
    html! {
        header #header {
            a href="/" {
                h1 { "david's " (get_word()) }
            }
            h2 { (get_tagline()) }
            nav {
                a href="/about" { "about" }
                a href="https://hachyderm.io/@dathagerty" rel="me" { "mastodon" }
            }
        }
    }
}

pub(crate) fn index() -> Markup {
    html! {
        (DOCTYPE)
        (head())
        (header())
        "wow content"
    }
}
