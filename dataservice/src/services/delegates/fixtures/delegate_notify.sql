create table delegates (
    id serial primary key,
    name text not null
);

create table somes_user (
    id serial primary key,
    email varchar(255) not null,
    is_email_hashed boolean not null default false
);

create table user_notification_settings (
    id bigint generated always as identity primary key,
    user_id int not null references somes_user(id) on delete cascade,
    platform varchar(20) not null,
    send_new_delegate_activity boolean not null default true,
    unique (user_id, platform)
);

create table expo_push_tokens (
    id bigint generated always as identity primary key,
    user_id int not null references somes_user(id) on delete cascade,
    notification_settings_id bigint not null references user_notification_settings(id) on delete cascade,
    push_token varchar(255) not null,
    platform varchar(20) not null,
    enabled boolean not null default true,
    unique (push_token)
);

create table favo_dels (
    id serial primary key,
    user_id int not null references somes_user(id) on delete cascade,
    delegate_id int not null references delegates(id) on delete cascade,
    favo_on timestamptz default now(),
    user_info_days int not null default 30
);

create table delegate_activity_email_info (
    id serial primary key,
    user_id int not null references somes_user(id) on delete cascade,
    delegate_id int not null references delegates(id) on delete cascade,
    last_sent_at timestamptz not null default now(),
    unique (user_id, delegate_id)
);
