use tracing::instrument;

#[instrument]
pub(crate) fn get_word() -> &'static str {
    "deliriums"
}

#[instrument]
pub(crate) fn get_tagline() -> &'static str {
    "a little rusty"
}
