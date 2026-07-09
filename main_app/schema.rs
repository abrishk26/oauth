// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Uuid,
        user_id -> Uuid,
        provider -> Text,
        password -> Nullable<Text>,
        provider_id -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        name -> Text,
        email -> Text,
        email_verified -> Bool,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(accounts -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(accounts, users,);
