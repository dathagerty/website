use topcoat::{
    Result,
    view::{Unescaped, component, view},
};

use crate::content::{GoModule, Page, Post, TagSummary};

use super::{
    module_instructions, module_list, post_list, post_metadata, post_year_list, tag_summary_list,
};

#[component]
pub async fn markdown_page(page: Page) -> Result {
    view! {
        <article class="markdown-page">
            <h2>(page.title)</h2>
            (Unescaped::new_unchecked(page.html))
        </article>
    }
}

#[component]
pub async fn blog_index(years: Vec<(i32, Vec<Post>)>) -> Result {
    view! {
        <section>
            <h2>
                "My various "
                <s>"writings"</s>
                " ramblings"
            </h2>
            <p>
                "You probably don't really care about this page; just click the unvisited links."
            </p>
            post_year_list(years: years)
        </section>
    }
}

#[component]
pub async fn post_page(post: Post) -> Result {
    let metadata_post = post.clone();

    view! {
        <article class="blog-post">
            <h2>(post.title)</h2>
            post_metadata(post: metadata_post)
            <hr>
            <div class="markdown-body">(Unescaped::new_unchecked(post.html))</div>
        </article>
    }
}

#[component]
pub async fn tags_index(tags: Vec<TagSummary>) -> Result {
    view! {
        <section>
            <h2>"Tags"</h2>
            tag_summary_list(tags: tags)
        </section>
    }
}

#[component]
pub async fn tag_page(tag: String, posts: Vec<Post>) -> Result {
    view! {
        <section>
            <h2>
                "Posts tagged "
                (tag)
            </h2>
            post_list(posts: posts)
        </section>
    }
}

#[component]
pub async fn modules_index(modules: Vec<GoModule>) -> Result {
    view! {
        <section>
            <h2>"Go Modules"</h2>
            <p>
                "I've written some Go modules, listed here. They range from pretty useful to "
                <q>"oh God, why"</q>
                " in quality, so user beware."
            </p>
            <p>"Anyways, here they are."</p>
            module_list(modules: modules)
        </section>
    }
}

#[component]
pub async fn module_page(module: GoModule) -> Result {
    let instructions_module = module.clone();

    view! {
        <article class="go-module">
            <h2>(module.name)</h2>
            <p>(module.description)</p>
            module_instructions(module: instructions_module)
        </article>
    }
}

#[component]
pub async fn not_found(path: &str) -> Result {
    view! {
        <section class="error-page">
            <h2>"Not all who wander are lost..."</h2>
            <p>
                "But you sure are; the page requested at "
                <code>(path)</code>
                " does not exist."
            </p>
        </section>
    }
}

#[component]
pub async fn internal_error() -> Result {
    view! {
        <section class="error-page">
            <h2>"Something went wrong"</h2>
            <p>"The server wandered into the void. Please try again later."</p>
        </section>
    }
}
