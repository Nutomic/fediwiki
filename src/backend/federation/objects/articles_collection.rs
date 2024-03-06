use crate::{
    backend::{database::IbisData, error::Error, federation::objects::article::ApubArticle},
    common::{DbArticle, DbInstance},
};
use activitypub_federation::{
    config::Data,
    kinds::collection::CollectionType,
    protocol::verification::verify_domains_match,
    traits::{Collection, Object},
};
use futures::{future, future::try_join_all};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCollection {
    pub r#type: CollectionType,
    pub id: Url,
    pub total_items: i32,
    pub items: Vec<ApubArticle>,
}

#[derive(Clone, Debug)]
pub struct DbArticleCollection(Vec<DbArticle>);

#[async_trait::async_trait]
impl Collection for DbArticleCollection {
    type Owner = DbInstance;
    type DataType = IbisData;
    type Kind = ArticleCollection;
    type Error = Error;

    async fn read_local(
        owner: &Self::Owner,
        data: &Data<Self::DataType>,
    ) -> Result<Self::Kind, Self::Error> {
        let local_articles = DbArticle::read_all(true, data)?;
        let articles = future::try_join_all(
            local_articles
                .into_iter()
                .map(|a| a.into_json(data))
                .collect::<Vec<_>>(),
        )
        .await?;
        let collection = ArticleCollection {
            r#type: Default::default(),
            id: owner.articles_url.clone().into(),
            total_items: articles.len() as i32,
            items: articles,
        };
        Ok(collection)
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(&json.id, expected_domain)?;
        Ok(())
    }

    async fn from_json(
        apub: Self::Kind,
        _owner: &Self::Owner,
        data: &Data<Self::DataType>,
    ) -> Result<Self, Self::Error> {
        let articles = try_join_all(
            apub.items
                .into_iter()
                .map(|i| DbArticle::from_json(i, data)),
        )
        .await?;

        // TODO: return value propably not needed
        Ok(DbArticleCollection(articles))
    }
}
