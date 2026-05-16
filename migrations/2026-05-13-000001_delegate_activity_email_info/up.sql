create table delegate_activity_email_info(
    id serial primary key,
    user_id int not null,
    delegate_id int not null,
    last_sent_at timestamptz not null default now(),
    foreign key (user_id) references somes_user(id) on delete cascade,
    foreign key (delegate_id) references delegates(id) on delete cascade,
    unique(user_id, delegate_id)
);
