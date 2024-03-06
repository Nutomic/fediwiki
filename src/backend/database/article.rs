use crate::{
    backend::{
        database::{
            schema::{article, edit, instance},
            IbisData,
        },
        error::MyResult,
        federation::objects::edits_collection::DbEditCollection,
    },
    common::{ArticleView, DbArticle, DbEdit, EditVersion},
};
use activitypub_federation::fetch::{collection_id::CollectionId, object_id::ObjectId};
use diesel::{
    dsl::max,
    insert_into,
    AsChangeset,
    BoolExpressionMethods,
    ExpressionMethods,
    Insertable,
    PgTextExpressionMethods,
    QueryDsl,
    RunQueryDsl,
};
use std::ops::DerefMut;

#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = article, check_for_backend(diesel::pg::Pg))]
pub struct DbArticleForm {
    pub title: String,
    pub text: String,
    pub ap_id: ObjectId<DbArticle>,
    pub instance_id: i32,
    pub local: bool,
    pub protected: bool,
}

// TODO: get rid of unnecessary methods
impl DbArticle {
    pub fn edits_id(&self) -> MyResult<CollectionId<DbEditCollection>> {
        Ok(CollectionId::parse(&format!("{}/edits", self.ap_id))?)
    }

    pub fn create(mut form: DbArticleForm, data: &IbisData) -> MyResult<Self> {
        form.title = form.title.replace(' ', "_");
        let mut conn = data.db_pool.get()?;
        Ok(insert_into(article::table)
            .values(form)
            .get_result(conn.deref_mut())?)
    }

    pub fn create_or_update(mut form: DbArticleForm, data: &IbisData) -> MyResult<Self> {
        form.title = form.title.replace(' ', "_");
        let mut conn = data.db_pool.get()?;
        Ok(insert_into(article::table)
            .values(&form)
            .on_conflict(article::dsl::ap_id)
            .do_update()
            .set(&form)
            .get_result(conn.deref_mut())?)
    }

    pub fn update_text(id: i32, text: &str, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(diesel::update(article::dsl::article.find(id))
            .set(article::dsl::text.eq(text))
            .get_result::<Self>(conn.deref_mut())?)
    }

    pub fn update_protected(id: i32, locked: bool, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(diesel::update(article::dsl::article.find(id))
            .set(article::dsl::protected.eq(locked))
            .get_result::<Self>(conn.deref_mut())?)
    }

    pub fn read(id: i32, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(article::table.find(id).get_result(conn.deref_mut())?)
    }

    pub fn read_view(id: i32, data: &IbisData) -> MyResult<ArticleView> {
        let mut conn = data.db_pool.get()?;
        let article: DbArticle = { article::table.find(id).get_result(conn.deref_mut())? };
        let latest_version = article.latest_edit_version(data)?;
        let edits = DbEdit::read_for_article(&article, data)?;
        Ok(ArticleView {
            article,
            edits,
            latest_version,
        })
    }

    pub fn read_view_title(
        title: &str,
        domain: Option<String>,
        data: &IbisData,
    ) -> MyResult<ArticleView> {
        let mut conn = data.db_pool.get()?;
        let article: DbArticle = {
            let query = article::table
                .inner_join(instance::table)
                .filter(article::dsl::title.eq(title))
                .into_boxed();
            let query = if let Some(domain) = domain {
                query
                    .filter(instance::dsl::domain.eq(domain))
                    .filter(instance::dsl::local.eq(false))
            } else {
                query.filter(article::dsl::local.eq(true))
            };
            query
                .select(article::all_columns)
                .get_result(conn.deref_mut())?
        };
        let latest_version = article.latest_edit_version(data)?;
        let edits = DbEdit::read_for_article(&article, data)?;
        Ok(ArticleView {
            article,
            edits,
            latest_version,
        })
    }

    pub fn read_from_ap_id(ap_id: &ObjectId<DbArticle>, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(article::table
            .filter(article::dsl::ap_id.eq(ap_id))
            .get_result(conn.deref_mut())?)
    }

    pub fn read_local_title(title: &str, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(article::table
            .filter(article::dsl::title.eq(title))
            .filter(article::dsl::local.eq(true))
            .get_result(conn.deref_mut())?)
    }

    /// Read all articles, ordered by most recently edited first.
    pub fn read_all(only_local: bool, data: &IbisData) -> MyResult<Vec<Self>> {
        let mut conn = data.db_pool.get()?;
        let query = article::table
            .inner_join(edit::table)
            .group_by(article::dsl::id)
            .order_by(max(edit::dsl::created).desc())
            .select(article::all_columns);
        Ok(if only_local {
            query
                .filter(article::dsl::local.eq(true))
                .get_results(&mut conn)?
        } else {
            query.get_results(&mut conn)?
        })
    }

    pub fn search(query: &str, data: &IbisData) -> MyResult<Vec<Self>> {
        let mut conn = data.db_pool.get()?;
        let replaced = query
            .replace('%', "\\%")
            .replace('_', "\\_")
            .replace(' ', "%");
        let replaced = format!("%{replaced}%");
        Ok(article::table
            .filter(
                article::dsl::title
                    .ilike(&replaced)
                    .or(article::dsl::text.ilike(&replaced)),
            )
            .get_results(conn.deref_mut())?)
    }

    pub fn latest_edit_version(&self, data: &IbisData) -> MyResult<EditVersion> {
        let mut conn = data.db_pool.get()?;
        let latest_version: Option<EditVersion> = edit::table
            .filter(edit::dsl::article_id.eq(self.id))
            .order_by(edit::dsl::id.desc())
            .limit(1)
            .select(edit::dsl::hash)
            .get_result(conn.deref_mut())
            .ok();
        match latest_version {
            Some(latest_version) => Ok(latest_version),
            None => Ok(EditVersion::default()),
        }
    }
}
