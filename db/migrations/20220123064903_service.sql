-- Add migration script here
CREATE table services(
    id integer not null primary key,
    created_at datetime not null default current_timestamp,
    updated_at datetime,
    deleted_at datetime,

    host text not null,
    port integer not null
)

