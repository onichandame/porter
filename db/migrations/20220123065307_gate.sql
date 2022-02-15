-- Add migration script here
CREATE table gates(
    id integer not null primary key,
    created_at datetime not null default current_timestamp,
    updated_at datetime,
    deleted_at datetime,

    service_id integer not null,
    host text not null,
    port integer not null,

    FOREIGN KEY(service_id) REFERENCES services(id)
)

