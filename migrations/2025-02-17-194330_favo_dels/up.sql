-- Your SQL goes here
create table favo_dels(
    id serial primary key,
    user_id int not null,
    delegate_id int not null,
    favo_on timestamp default now(),
    foreign key (user_id) references somes_user(id),
    foreign key (delegate_id) references delegates(id)
);