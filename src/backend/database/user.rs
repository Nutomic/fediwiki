use crate::{
    backend::{
        database::{
            schema::{instance, instance_follow, local_user, person},
            IbisData,
        },
        error::MyResult,
    },
    common::{utils::http_protocol_str, DbInstance, DbLocalUser, DbPerson, LocalUserView},
};
use activitypub_federation::{
    config::Data,
    fetch::object_id::ObjectId,
    http_signatures::generate_actor_keypair,
};
use bcrypt::{hash, DEFAULT_COST};
use chrono::{DateTime, Local, Utc};
use diesel::{
    insert_into,
    AsChangeset,
    ExpressionMethods,
    Insertable,
    JoinOnDsl,
    PgTextExpressionMethods,
    QueryDsl,
    RunQueryDsl,
};
use std::ops::DerefMut;

#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = local_user, check_for_backend(diesel::pg::Pg))]
pub struct DbLocalUserForm {
    pub password_encrypted: String,
    pub person_id: i32,
    pub admin: bool,
}

#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = person, check_for_backend(diesel::pg::Pg))]
pub struct DbPersonForm {
    pub username: String,
    pub ap_id: ObjectId<DbPerson>,
    pub inbox_url: String,
    pub public_key: String,
    pub private_key: Option<String>,
    pub last_refreshed_at: DateTime<Utc>,
    pub local: bool,
}

impl DbPerson {
    pub fn create(person_form: &DbPersonForm, data: &Data<IbisData>) -> MyResult<Self> {
        let mut conn = data.db_pool.get()?;
        Ok(insert_into(person::table)
            .values(person_form)
            .on_conflict(person::dsl::ap_id)
            .do_update()
            .set(person_form)
            .get_result::<DbPerson>(conn.deref_mut())?)
    }

    pub fn read(id: i32, data: &Data<IbisData>) -> MyResult<DbPerson> {
        let mut conn = data.db_pool.get()?;
        Ok(person::table.find(id).get_result(conn.deref_mut())?)
    }

    pub fn create_local(
        username: String,
        password: String,
        admin: bool,
        data: &IbisData,
    ) -> MyResult<LocalUserView> {
        let mut conn = data.db_pool.get()?;
        let domain = &data.config.federation.domain;
        let ap_id = ObjectId::parse(&format!(
            "{}://{domain}/user/{username}",
            http_protocol_str()
        ))?;
        let inbox_url = format!("{}://{domain}/inbox", http_protocol_str());
        let keypair = generate_actor_keypair()?;
        let person_form = DbPersonForm {
            username,
            ap_id,
            inbox_url,
            public_key: keypair.public_key,
            private_key: Some(keypair.private_key),
            last_refreshed_at: Local::now().into(),
            local: true,
        };

        let person = insert_into(person::table)
            .values(person_form)
            .get_result::<DbPerson>(conn.deref_mut())?;

        let local_user_form = DbLocalUserForm {
            password_encrypted: hash(password, DEFAULT_COST)?,
            person_id: person.id,
            admin,
        };

        let local_user = insert_into(local_user::table)
            .values(local_user_form)
            .get_result::<DbLocalUser>(conn.deref_mut())?;

        Ok(LocalUserView {
            local_user,
            person,
            following: vec![],
        })
    }

    pub fn read_from_ap_id(
        ap_id: &ObjectId<DbPerson>,
        data: &Data<IbisData>,
    ) -> MyResult<DbPerson> {
        let mut conn = data.db_pool.get()?;
        Ok(person::table
            .filter(person::dsl::ap_id.eq(ap_id))
            .get_result(conn.deref_mut())?)
    }

    pub fn read_from_name(
        username: &str,
        domain: &Option<String>,
        data: &Data<IbisData>,
    ) -> MyResult<DbPerson> {
        let mut conn = data.db_pool.get()?;
        let mut query = person::table
            .filter(person::username.eq(username))
            .select(person::all_columns)
            .into_boxed();
        query = if let Some(domain) = domain {
            let domain_pattern = format!("{}://{domain}/%", http_protocol_str());
            query
                .filter(person::ap_id.ilike(domain_pattern))
                .filter(person::local.eq(false))
        } else {
            query.filter(person::local.eq(true))
        };
        Ok(query.get_result(conn.deref_mut())?)
    }

    pub fn read_local_from_name(username: &str, data: &Data<IbisData>) -> MyResult<LocalUserView> {
        let mut conn = data.db_pool.get()?;
        let (person, local_user) = person::table
            .inner_join(local_user::table)
            .filter(person::dsl::local)
            .filter(person::dsl::username.eq(username))
            .get_result::<(DbPerson, DbLocalUser)>(conn.deref_mut())?;
        // TODO: handle this in single query
        let following = Self::read_following(person.id, data)?;
        Ok(LocalUserView {
            person,
            local_user,
            following,
        })
    }

    fn read_following(id_: i32, data: &Data<IbisData>) -> MyResult<Vec<DbInstance>> {
        use instance_follow::dsl::{follower_id, instance_id};
        let mut conn = data.db_pool.get()?;
        Ok(instance_follow::table
            .inner_join(instance::table.on(instance_id.eq(instance::dsl::id)))
            .filter(follower_id.eq(id_))
            .select(instance::all_columns)
            .get_results(conn.deref_mut())?)
    }
}
