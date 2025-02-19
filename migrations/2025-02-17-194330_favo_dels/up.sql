-- Your SQL goes here
create table favo_dels(
    id serial primary key,
    user_id int not null,
    delegate_id int not null,
    favo_on timestamp default now(),
    foreign key (user_id) references somes_user(id),
    foreign key (delegate_id) references delegates(id)
);

create table favo_legis_inits(
    id serial primary key,
    user_id int not null,
    legis_init_id int not null,
    favo_on timestamp default now(),
    foreign key (user_id) references somes_user(id),
    foreign key (legis_init_id) references legislative_initiatives(id)
);
