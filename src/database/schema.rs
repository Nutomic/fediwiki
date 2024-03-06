// @generated automatically by Diesel CLI.

diesel::table! {
    article (id) {
        id -> Int4,
        title -> Text,
        text -> Text,
        #[max_length = 255]
        ap_id -> Varchar,
        instance_id -> Int4,
        local -> Bool,
        protected -> Bool,
    }
}

diesel::table! {
    conflict (id) {
        id -> Int4,
        hash -> Uuid,
        diff -> Text,
        summary -> Text,
        creator_id -> Int4,
        article_id -> Int4,
        previous_version_id -> Uuid,
    }
}

diesel::table! {
    edit (id) {
        id -> Int4,
        creator_id -> Int4,
        hash -> Uuid,
        #[max_length = 255]
        ap_id -> Varchar,
        diff -> Text,
        summary -> Text,
        article_id -> Int4,
        previous_version_id -> Uuid,
        created -> Timestamptz,
    }
}

diesel::table! {
    instance (id) {
        id -> Int4,
        domain -> Text,
        #[max_length = 255]
        ap_id -> Varchar,
        description -> Nullable<Text>,
        inbox_url -> Text,
        #[max_length = 255]
        articles_url -> Varchar,
        public_key -> Text,
        private_key -> Nullable<Text>,
        last_refreshed_at -> Timestamptz,
        local -> Bool,
    }
}

diesel::table! {
    instance_follow (id) {
        id -> Int4,
        instance_id -> Int4,
        follower_id -> Int4,
        pending -> Bool,
    }
}

diesel::table! {
    jwt_secret (id) {
        id -> Int4,
        secret -> Varchar,
    }
}

diesel::table! {
    local_user (id) {
        id -> Int4,
        password_encrypted -> Text,
        person_id -> Int4,
        admin -> Bool,
    }
}

diesel::table! {
    person (id) {
        id -> Int4,
        username -> Text,
        #[max_length = 255]
        ap_id -> Varchar,
        inbox_url -> Text,
        public_key -> Text,
        private_key -> Nullable<Text>,
        last_refreshed_at -> Timestamptz,
        local -> Bool,
    }
}

diesel::joinable!(article -> instance (instance_id));
diesel::joinable!(conflict -> article (article_id));
diesel::joinable!(conflict -> local_user (creator_id));
diesel::joinable!(edit -> article (article_id));
diesel::joinable!(edit -> person (creator_id));
diesel::joinable!(instance_follow -> instance (instance_id));
diesel::joinable!(instance_follow -> person (follower_id));
diesel::joinable!(local_user -> person (person_id));

diesel::allow_tables_to_appear_in_same_query!(
    article,
    conflict,
    edit,
    instance,
    instance_follow,
    jwt_secret,
    local_user,
    person,
);
