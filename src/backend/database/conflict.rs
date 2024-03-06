use crate::{
    backend::{
        database::{schema::conflict, IbisData},
        error::MyResult,
        federation::activities::submit_article_update,
        utils::generate_article_version,
    },
    common::{ApiConflict, DbArticle, DbEdit, DbLocalUser, EditVersion},
};
use activitypub_federation::config::Data;
use diesel::{
    delete,
    insert_into,
    ExpressionMethods,
    Identifiable,
    Insertable,
    QueryDsl,
    Queryable,
    RunQueryDsl,
    Selectable,
};
use diffy::{apply, merge, Patch};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;

/// A local only object which represents a merge conflict. It is created
/// when a local user edit conflicts with another concurrent edit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Queryable, Selectable, Identifiable)]
#[diesel(table_name = conflict, check_for_backend(diesel::pg::Pg), belongs_to(DbArticle, foreign_key = article_id))]
pub struct DbConflict {
    pub id: i32,
    pub hash: EditVersion,
    pub diff: String,
    pub summary: String,
    pub creator_id: i32,
    pub article_id: i32,
    pub previous_version_id: EditVersion,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = conflict, check_for_backend(diesel::pg::Pg))]
pub struct DbConflictForm {
    pub hash: EditVersion,
    pub diff: String,
    pub summary: String,
    pub creator_id: i32,
    pub article_id: i32,
    pub previous_version_id: EditVersion,
}

impl DbConflict {
    pub fn create(form: &DbConflictForm, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(insert_into(conflict::table)
            .values(form)
            .get_result(conn.deref_mut())?)
    }

    pub fn list(local_user: &DbLocalUser, data: &IbisData) -> MyResult<Vec<Self>> {
        let mut conn = data.db_pool.get()?;
        Ok(conflict::table
            .filter(conflict::dsl::creator_id.eq(local_user.id))
            .get_results(conn.deref_mut())?)
    }

    /// Delete a merge conflict after it is resolved.
    pub fn delete(id: i32, data: &IbisData) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(delete(conflict::table.find(id)).get_result(conn.deref_mut())?)
    }

    pub async fn to_api_conflict(&self, data: &Data<IbisData>) -> MyResult<Option<ApiConflict>> {
        let article = DbArticle::read(self.article_id, data)?;
        // Make sure to get latest version from origin so that all conflicts can be resolved
        let original_article = article.ap_id.dereference_forced(data).await?;

        // create common ancestor version
        let edits = DbEdit::read_for_article(&original_article, data)?;
        let ancestor = generate_article_version(&edits, &self.previous_version_id)?;

        let patch = Patch::from_str(&self.diff)?;
        // apply self.diff to ancestor to get `ours`
        let ours = apply(&ancestor, &patch)?;
        match merge(&ancestor, &ours, &original_article.text) {
            Ok(new_text) => {
                // patch applies cleanly so we are done
                // federate the change
                submit_article_update(
                    new_text,
                    self.summary.clone(),
                    self.previous_version_id.clone(),
                    &original_article,
                    self.creator_id,
                    data,
                )
                .await?;
                DbConflict::delete(self.id, data)?;
                Ok(None)
            }
            Err(three_way_merge) => {
                // there is a merge conflict, user needs to do three-way-merge
                Ok(Some(ApiConflict {
                    id: self.id,
                    hash: self.hash.clone(),
                    three_way_merge,
                    summary: self.summary.clone(),
                    article: original_article.clone(),
                    previous_version_id: original_article.latest_edit_version(data)?,
                }))
            }
        }
    }
}
