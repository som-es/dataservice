-- Your SQL goes here
create table favo_dels(
    id serial primary key,
    user_id int not null,
    delegate_id int not null,
    favo_on timestamptz default now(),
    foreign key (user_id) references somes_user(id) on delete cascade,
    foreign key (delegate_id) references delegates(id) on delete cascade
);

create table favo_legis_inits(
    id serial primary key,
    user_id int not null,
    legis_init_id int not null,
    favo_on timestamptz default now(),
    foreign key (user_id) references somes_user(id) on delete cascade,
    foreign key (legis_init_id) references legislative_initiatives(id) on delete cascade
);
