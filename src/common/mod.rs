pub mod newtypes;
pub mod utils;
pub mod validation;

use chrono::{DateTime, Utc};
use newtypes::{ArticleId, ConflictId, EditId, InstanceId, PersonId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use smart_default::SmartDefault;
use url::Url;
use uuid::Uuid;
#[cfg(feature = "ssr")]
use {
    crate::backend::{
        database::schema::{article, edit, instance, local_user, person},
        federation::objects::articles_collection::DbArticleCollection,
        federation::objects::instance_collection::DbInstanceCollection,
    },
    activitypub_federation::fetch::{collection_id::CollectionId, object_id::ObjectId},
    diesel::{Identifiable, Queryable, Selectable},
    doku::Document,
};

pub const MAIN_PAGE_NAME: &str = "Main_Page";

/// Should be an enum Title/Id but fails due to https://github.com/nox/serde_urlencoded/issues/66
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GetArticleForm {
    pub title: Option<String>,
    pub domain: Option<String>,
    pub id: Option<ArticleId>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ListArticlesForm {
    pub only_local: Option<bool>,
    pub instance_id: Option<InstanceId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
#[cfg_attr(feature = "ssr", diesel(table_name = article, check_for_backend(diesel::pg::Pg)))]
pub struct ArticleView {
    pub article: DbArticle,
    pub latest_version: EditVersion,
    pub edits: Vec<EditView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable, Selectable, Identifiable))]
#[cfg_attr(feature = "ssr", diesel(table_name = article, check_for_backend(diesel::pg::Pg), belongs_to(DbInstance, foreign_key = instance_id)))]
pub struct DbArticle {
    pub id: ArticleId,
    pub title: String,
    pub text: String,
    #[cfg(feature = "ssr")]
    pub ap_id: ObjectId<DbArticle>,
    #[cfg(not(feature = "ssr"))]
    pub ap_id: String,
    pub instance_id: InstanceId,
    pub local: bool,
    pub protected: bool,
    pub approved: bool,
    pub published: DateTime<Utc>,
}

/// Represents a single change to the article.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable, Selectable))]
#[cfg_attr(feature = "ssr", diesel(table_name = edit, check_for_backend(diesel::pg::Pg)))]
pub struct DbEdit {
    // TODO: we could use hash as primary key, but that gives errors on forking because
    //       the same edit is used for multiple articles
    pub id: EditId,
    #[serde(skip)]
    pub creator_id: PersonId,
    /// UUID built from sha224 hash of diff
    pub hash: EditVersion,
    #[cfg(feature = "ssr")]
    pub ap_id: ObjectId<DbEdit>,
    #[cfg(not(feature = "ssr"))]
    pub ap_id: String,
    pub diff: String,
    pub summary: String,
    pub article_id: ArticleId,
    /// First edit of an article always has `EditVersion::default()` here
    pub previous_version_id: EditVersion,
    pub published: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
#[cfg_attr(feature = "ssr", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct EditView {
    pub edit: DbEdit,
    pub creator: DbPerson,
}

/// The version hash of a specific edit. Generated by taking an SHA256 hash of the diff
/// and using the first 16 bytes so that it fits into UUID.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ssr", derive(diesel_derive_newtype::DieselNewType))]
pub struct EditVersion(pub(crate) Uuid);

impl EditVersion {
    pub fn new(diff: &str) -> Self {
        let mut sha256 = Sha256::new();
        sha256.update(diff);
        let hash_bytes = sha256.finalize();
        let uuid =
            Uuid::from_slice(&hash_bytes.as_slice()[..16]).expect("hash is correct size for uuid");
        EditVersion(uuid)
    }

    pub fn hash(&self) -> String {
        hex::encode(self.0.into_bytes())
    }
}

impl Default for EditVersion {
    fn default() -> Self {
        EditVersion::new("")
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RegisterUserForm {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct LoginUserForm {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, SmartDefault)]
#[serde(default)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "ssr", derive(Queryable, Document))]
#[cfg_attr(feature = "ssr", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct SharedConfig {
    /// Whether users can create new accounts
    #[default = true]
    #[cfg_attr(feature = "ssr", doku(example = "true"))]
    pub registration_open: bool,
    /// Whether admins need to approve new articles
    #[default = false]
    #[cfg_attr(feature = "ssr", doku(example = "false"))]
    pub article_approval: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
#[cfg_attr(feature = "ssr", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct SiteView {
    pub my_profile: Option<LocalUserView>,
    pub config: SharedConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
#[cfg_attr(feature = "ssr", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct LocalUserView {
    pub person: DbPerson,
    pub local_user: DbLocalUser,
    pub following: Vec<DbInstance>,
}

/// A user with account registered on local instance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable, Selectable, Identifiable))]
#[cfg_attr(feature = "ssr", diesel(table_name = local_user, check_for_backend(diesel::pg::Pg)))]
pub struct DbLocalUser {
    pub id: InstanceId,
    #[serde(skip)]
    pub password_encrypted: String,
    pub person_id: PersonId,
    pub admin: bool,
}

/// Federation related data from a local or remote user.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable, Selectable, Identifiable))]
#[cfg_attr(feature = "ssr", diesel(table_name = person, check_for_backend(diesel::pg::Pg)))]
pub struct DbPerson {
    pub id: PersonId,
    pub username: String,
    #[cfg(feature = "ssr")]
    pub ap_id: ObjectId<DbPerson>,
    #[cfg(not(feature = "ssr"))]
    pub ap_id: String,
    pub inbox_url: String,
    #[serde(skip)]
    pub public_key: String,
    #[serde(skip)]
    pub private_key: Option<String>,
    #[serde(skip)]
    pub last_refreshed_at: DateTime<Utc>,
    pub local: bool,
}

impl DbPerson {
    pub fn inbox_url(&self) -> Url {
        Url::parse(&self.inbox_url).expect("can parse inbox url")
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateArticleForm {
    pub title: String,
    pub text: String,
    pub summary: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EditArticleForm {
    /// Id of the article to edit
    pub article_id: ArticleId,
    /// Full, new text of the article. A diff against `previous_version` is generated on the backend
    /// side to handle conflicts.
    pub new_text: String,
    /// What was changed
    pub summary: String,
    /// The version that this edit is based on, ie [DbArticle.latest_version] or
    /// [ApiConflict.previous_version]
    pub previous_version_id: EditVersion,
    /// If you are resolving a conflict, pass the id to delete conflict from the database
    pub resolve_conflict_id: Option<ConflictId>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProtectArticleForm {
    pub article_id: ArticleId,
    pub protected: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ForkArticleForm {
    pub article_id: ArticleId,
    pub new_title: String,
}

#[derive(Deserialize, Serialize)]
pub struct ApproveArticleForm {
    pub article_id: ArticleId,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GetInstance {
    pub id: Option<InstanceId>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FollowInstance {
    pub id: InstanceId,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct SearchArticleForm {
    pub query: String,
}

#[derive(Deserialize, Serialize)]
pub struct ResolveObject {
    pub id: Url,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiConflict {
    pub id: ConflictId,
    pub hash: EditVersion,
    pub three_way_merge: String,
    pub summary: String,
    pub article: DbArticle,
    pub previous_version_id: EditVersion,
    pub published: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Notification {
    EditConflict(ApiConflict),
    ArticleApprovalRequired(DbArticle),
}

impl Notification {
    pub fn published(&self) -> &DateTime<Utc> {
        match self {
            Notification::EditConflict(api_conflict) => &api_conflict.published,
            Notification::ArticleApprovalRequired(db_article) => &db_article.published,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable, Selectable, Identifiable))]
#[cfg_attr(feature = "ssr", diesel(table_name = instance, check_for_backend(diesel::pg::Pg)))]
pub struct DbInstance {
    pub id: InstanceId,
    pub domain: String,
    #[cfg(feature = "ssr")]
    pub ap_id: ObjectId<DbInstance>,
    #[cfg(not(feature = "ssr"))]
    pub ap_id: String,
    pub description: Option<String>,
    #[cfg(feature = "ssr")]
    pub articles_url: Option<CollectionId<DbArticleCollection>>,
    #[cfg(not(feature = "ssr"))]
    pub articles_url: String,
    pub inbox_url: String,
    #[serde(skip)]
    pub public_key: String,
    #[serde(skip)]
    pub private_key: Option<String>,
    #[serde(skip)]
    pub last_refreshed_at: DateTime<Utc>,
    pub local: bool,
    #[cfg(feature = "ssr")]
    pub instances_url: Option<CollectionId<DbInstanceCollection>>,
}

impl DbInstance {
    pub fn inbox_url(&self) -> Url {
        Url::parse(&self.inbox_url).expect("can parse inbox url")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ssr", derive(Queryable))]
#[cfg_attr(feature = "ssr", diesel(table_name = article, check_for_backend(diesel::pg::Pg)))]
pub struct InstanceView {
    pub instance: DbInstance,
    pub followers: Vec<DbPerson>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GetUserForm {
    pub name: String,
    pub domain: Option<String>,
}

#[test]
fn test_edit_versions() {
    let default = EditVersion::default();
    assert_eq!("e3b0c44298fc1c149afbf4c8996fb924", default.hash());

    let version = EditVersion::new("test");
    assert_eq!("9f86d081884c7d659a2feaa0c55ad015", version.hash());
}
