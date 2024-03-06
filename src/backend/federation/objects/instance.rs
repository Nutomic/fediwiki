use crate::{
    backend::{
        database::{instance::DbInstanceForm, IbisData},
        error::{Error, MyResult},
        federation::{objects::articles_collection::DbArticleCollection, send_activity},
    },
    common::{utils::extract_domain, DbInstance},
};
use activitypub_federation::{
    config::Data,
    fetch::{collection_id::CollectionId, object_id::ObjectId},
    kinds::actor::ServiceType,
    protocol::{public_key::PublicKey, verification::verify_domains_match},
    traits::{ActivityHandler, Actor, Object},
};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApubInstance {
    #[serde(rename = "type")]
    kind: ServiceType,
    id: ObjectId<DbInstance>,
    content: Option<String>,
    articles: CollectionId<DbArticleCollection>,
    inbox: Url,
    public_key: PublicKey,
}

impl DbInstance {
    pub fn followers_url(&self) -> MyResult<Url> {
        Ok(Url::parse(&format!("{}/followers", self.ap_id.inner()))?)
    }

    pub fn follower_ids(&self, data: &Data<IbisData>) -> MyResult<Vec<Url>> {
        Ok(DbInstance::read_followers(self.id, data)?
            .into_iter()
            .map(|f| f.ap_id.into())
            .collect())
    }

    pub async fn send_to_followers<Activity>(
        &self,
        activity: Activity,
        extra_recipients: Vec<DbInstance>,
        data: &Data<IbisData>,
    ) -> Result<(), <Activity as ActivityHandler>::Error>
    where
        Activity: ActivityHandler + Serialize + Debug + Send + Sync,
        <Activity as ActivityHandler>::Error: From<activitypub_federation::error::Error>,
        <Activity as ActivityHandler>::Error: From<Error>,
    {
        let mut inboxes: Vec<_> = DbInstance::read_followers(self.id, data)?
            .iter()
            .map(|f| f.inbox_url())
            .collect();
        inboxes.extend(extra_recipients.into_iter().map(|i| i.inbox_url()));
        send_activity(self, activity, inboxes, data).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Object for DbInstance {
    type DataType = IbisData;
    type Kind = ApubInstance;
    type Error = Error;

    fn last_refreshed_at(&self) -> Option<DateTime<Utc>> {
        Some(self.last_refreshed_at)
    }

    async fn read_from_id(
        object_id: Url,
        data: &Data<Self::DataType>,
    ) -> Result<Option<Self>, Self::Error> {
        Ok(DbInstance::read_from_ap_id(&object_id.into(), data).ok())
    }

    async fn into_json(self, _data: &Data<Self::DataType>) -> Result<Self::Kind, Self::Error> {
        Ok(ApubInstance {
            kind: Default::default(),
            id: self.ap_id.clone(),
            content: self.description.clone(),
            articles: self.articles_url.clone(),
            inbox: Url::parse(&self.inbox_url)?,
            public_key: self.public_key(),
        })
    }

    async fn verify(
        json: &Self::Kind,
        expected_domain: &Url,
        _data: &Data<Self::DataType>,
    ) -> Result<(), Self::Error> {
        verify_domains_match(json.id.inner(), expected_domain)?;
        Ok(())
    }

    async fn from_json(json: Self::Kind, data: &Data<Self::DataType>) -> Result<Self, Self::Error> {
        let domain = extract_domain(&json.id);
        let form = DbInstanceForm {
            domain,
            ap_id: json.id,
            description: json.content,
            articles_url: json.articles,
            inbox_url: json.inbox.to_string(),
            public_key: json.public_key.public_key_pem,
            private_key: None,
            last_refreshed_at: Local::now().into(),
            local: false,
        };
        let instance = DbInstance::create(&form, data)?;
        // TODO: very inefficient to sync all articles every time
        instance.articles_url.dereference(&instance, data).await?;
        Ok(instance)
    }
}

impl Actor for DbInstance {
    fn id(&self) -> Url {
        self.ap_id.inner().clone()
    }

    fn public_key_pem(&self) -> &str {
        &self.public_key
    }

    fn private_key_pem(&self) -> Option<String> {
        self.private_key.clone()
    }

    fn inbox(&self) -> Url {
        self.inbox_url()
    }
}
