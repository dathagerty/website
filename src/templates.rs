use maud::{html, Markup, DOCTYPE};
use crate::flair::{get_tagline, get_word};
use tracing::{info, instrument};

const COMMIT_HASH: &str = build_info::format!("{} on {}", $.version_control?.git()?.commit_short_id, $.timestamp);

#[instrument]
pub fn root() -> Markup {
    info!("rendering root");
    html! {
        (DOCTYPE)
        (head("home".to_string()))
        (header())
        (page_content())
        (footer())
    }
}

#[instrument]
pub fn page() -> Markup {
    info!("rendering page");
    html! {
        (DOCTYPE)
        (head("page title".to_string()))
        (header())
        (page_content())
        (footer())
    }
}

#[instrument]
pub fn post(slug: String) -> Markup {
    info!("rendering post");
    html! {
        (DOCTYPE)
        (head("post title".to_string()))
        (header())
        (post_content(slug))
        (footer())
    }
}

#[instrument]
fn head(title: String) -> Markup {
    info!("rendering head");
    html! {
        head {
            meta http-equiv="Content-Type" content="text/html;charset=utf-8";
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            meta theme-color="#a3be8c";
            title { (title) }
        }
    }
}

#[instrument]
fn header() -> Markup {
    info!("rendering header");
    html! {
        header #header {
            a href="/" {
                h1 { "david's " (get_word()) }
            }
            h2 { (get_tagline()) }
            nav {
                a href="/about" { "about" }
                " "
                a href="/blag" { "blag" }
                " | "
                a href="https://hachyderm.io/@dathagerty" rel="me" { "mastodon" }
                " "
                a href="https://bsky.app/profile/dathagerty.com" rel="me" { "bsky" }
                " "
                a href="https://dathagerty.omg.lol" rel="me" { "omg.lol" }
                " "
                a href="https://github.com/dathagerty" rel="me" { "github" }
                " "
                a href="https://git.sr.ht/~gloatingfiddle" rel="me" { "sr.ht" }
            }
        }
    }
}

#[instrument]
pub fn footer() -> Markup {
    info!("rendering footer");
    html! {
        footer #footer {
            "Built with no emotion because it's automated from the code in commit "
            a href="https://github.com/dathagerty/website" {
                (COMMIT_HASH)
            }
        }
    }
}


#[instrument]
pub fn post_content(slug: String) -> Markup {
    info!(slug, "rendering post content");
    html! {
        "wow it's content for " (slug)
    }
}

#[instrument]
pub fn page_content() -> Markup {
    info!("rendering page content");
    html! {
        "wow it's a page"
    }
}

#[instrument]
fn page_link(href: &str, name: &str) -> Markup {
    info!(name, "creating link");
    html! {
        a href=(href) { (name) }
    }
}

#[instrument]
fn me_link(href: &str, name: &str) -> Markup {
    info!(name, "creating me link");
    html! {
        a href=(href) rel="me" { (name) }
    }
}
