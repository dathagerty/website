use async_trait::async_trait;

use super::{Branding, ContentError, GoModule, Page, Post, TagSummary};

#[async_trait]
pub trait ContentRepository: Send + Sync {
    async fn page(&self, slug: &str) -> Result<Option<Page>, ContentError>;
    async fn post(&self, slug: &str) -> Result<Option<Post>, ContentError>;
    async fn posts(&self) -> Result<Vec<Post>, ContentError>;
    async fn posts_by_year(&self) -> Result<Vec<(i32, Vec<Post>)>, ContentError>;
    async fn tags(&self) -> Result<Vec<TagSummary>, ContentError>;
    async fn posts_for_tag(&self, tag: &str) -> Result<Option<Vec<Post>>, ContentError>;
    async fn modules(&self) -> Result<Vec<GoModule>, ContentError>;
    async fn module(&self, name: &str) -> Result<Option<GoModule>, ContentError>;
    async fn branding(&self) -> Result<Branding, ContentError>;
}
