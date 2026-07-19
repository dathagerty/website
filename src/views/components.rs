use chrono::{Datelike, Utc};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use topcoat::{
    Result,
    view::{component, view},
};

use crate::content::{Branding, GoModule, Post, TagSummary};

const REPOSITORY_URL: &str = "https://github.com/dathagerty/website";
const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

build_info::build_info!(fn build_info);

fn commit_hash() -> Option<&'static str> {
    let build_info_hash = build_info()
        .version_control
        .as_ref()
        .and_then(build_info::VersionControl::git)
        .map(|git| git.commit_short_id.as_str());
    select_commit_hash(option_env!("RAILWAY_GIT_COMMIT_SHA"), build_info_hash)
}

pub(crate) fn select_commit_hash<'a>(
    railway_hash: Option<&'a str>,
    build_info_hash: Option<&'a str>,
) -> Option<&'a str> {
    railway_hash
        .filter(|hash| plausible_commit_hash(hash))
        .or_else(|| build_info_hash.filter(|hash| plausible_commit_hash(hash)))
}

fn plausible_commit_hash(hash: &str) -> bool {
    (7..=64).contains(&hash.len()) && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn tag_href(tag: &str) -> String {
    format!(
        "/tags/{}",
        utf8_percent_encode(tag, PATH_SEGMENT_ENCODE_SET)
    )
}

#[component]
pub async fn identity_links() -> Result {
    view! {
        <span class="identity-links">
            <a href="https://github.com/dathagerty" rel="me">"GitHub"</a>
            " "
            <a href="https://git.sr.ht/~gloatingfiddle" rel="me">"sr.ht"</a>
            " "
            <a href="https://hachyderm.io/@dathagerty" rel="me">"Mastodon"</a>
            " "
            <a href="https://bsky.app/profile/dathagerty.com" rel="me">"Bluesky"</a>
            " "
            <a href="https://dathagerty.omg.lol" rel="me">"omg.lol"</a>
        </span>
    }
}

#[component]
pub async fn navigation() -> Result {
    view! {
        <nav aria-label="Primary navigation">
            <a href="/about">"About"</a>
            " "
            <a href="/blag">"Blag"</a>
            " "
            <a href="/reading">"Reading"</a>
            " "
            <a href="/tags">"Tags"</a>
            " "
            <a href="/go">"Go"</a>
            " | "
            identity_links()
        </nav>
    }
}

#[component]
pub async fn site_header(branding: Branding) -> Result {
    view! {
        <header id="header">
            <a class="brand" href="/">
                <h1>
                    "david's "
                    (branding.word)
                </h1>
            </a>
            <p class="slogan">(branding.slogan)</p>
            navigation()
            <div class="section-break" aria-hidden="true"><span>"\u{00a7}"</span></div>
        </header>
    }
}

#[component]
pub async fn site_footer() -> Result {
    let year = Utc::now().year();

    view! {
        <footer id="footer">
            <p>
                "The contents of this site are licensed under "
                <a rel="license" href="https://creativecommons.org/licenses/by-sa/4.0/">
                    "CC BY-SA 4.0"
                </a>
                " as of "
                (year)
                "."
            </p>
            <p>
                {
                    build_commit(commit_hash: commit_hash())
                }
            </p>
        </footer>
    }
}

#[component]
pub async fn build_commit(commit_hash: Option<&str>) -> Result {
    view! {
        "Build commit: "
        if let Some(commit_hash) = commit_hash {
            <a href=(format!("{REPOSITORY_URL}/commit/{commit_hash}"))>(commit_hash)</a>
        } else {
            "unknown"
        }
    }
}

#[component]
pub async fn post_metadata(post: Post) -> Result {
    let publish_date = post.publish_date.to_string();
    let last_edit = post.last_edit.map(|date| date.to_string());

    view! {
        <div class="post-metadata">
            <p class="publish-dates">
                "Published on "
                <time datetime=(&publish_date)>(&publish_date)</time>
                if let Some(last_edit) = last_edit {
                    ", last edited on "
                    <time datetime=(&last_edit)>(&last_edit)</time>
                }
            </p>
            if !post.tags.is_empty() {
                <p class="tags">
                    "Tags: { "
                    for (index, tag) in post.tags.iter().enumerate() {
                        if index > 0 {
                            ", "
                        }
                        <a href=(tag_href(tag))>(tag)</a>
                    }
                    " }"
                </p>
            }
        </div>
    }
}

#[component]
pub async fn post_list(posts: Vec<Post>) -> Result {
    view! {
        <ul class="post-list">
            for post in posts {
                <li>
                    <a href=(format!("/blag/{}", post.slug))>(post.title)</a>
                    if let Some(summary) = post.summary {
                        <br>
                        <span class="post-summary">(summary)</span>
                    }
                </li>
            }
        </ul>
    }
}

#[component]
pub async fn post_year_list(years: Vec<(i32, Vec<Post>)>) -> Result {
    view! {
        for (year, posts) in years {
            <section class="post-year">
                <h3>(year)</h3>
                post_list(posts: posts)
            </section>
        }
    }
}

#[component]
pub async fn tag_summary_list(tags: Vec<TagSummary>) -> Result {
    view! {
        <ul class="tag-list">
            for tag in tags {
                <li>
                    <a href=(tag_href(&tag.name))>
                        (tag.name)
                        " ("
                        (tag.post_count)
                        ")"
                    </a>
                </li>
            }
        </ul>
    }
}

#[component]
pub async fn module_list(modules: Vec<GoModule>) -> Result {
    view! {
        <ul class="module-list">
            for module in modules {
                <li>
                    <a href=(format!("/go/{}", module.name))>(module.name)</a>
                    ": "
                    (module.description)
                </li>
            }
        </ul>
    }
}

#[component]
pub async fn module_instructions(module: GoModule) -> Result {
    let package_url = format!("https://pkg.go.dev/{}", module.module_path);
    let godocs_url = format!("https://godocs.io/{}", module.module_path);
    let license_url = format!("{}/tree/main/item/LICENSE", module.repository_url);
    let get_command = format!("$ go get {}", module.module_path);
    let import = format!("import \"{}\"", module.module_path);

    view! {
        <p>
            "View this package's "
            <a href=(&module.repository_url)>"source"</a>
            "."
        </p>
        <p>"Add it to your project:"</p>
        <pre class="shell"><code>(get_command)</code></pre>
        <p>"And then import it into your code:"</p>
        <pre class="golang"><code>(import)</code></pre>
        <p>"View documentation for this module:"</p>
        <ul>
            <li><a href=(package_url)>"On pkg.go.dev"</a></li>
            <li><a href=(godocs_url)>"On godocs.io"</a></li>
        </ul>
        <p>
            "The code in this module is available under the "
            <a href=(license_url)>
                (module.license)
                " license"
            </a>
            "."
        </p>
    }
}
