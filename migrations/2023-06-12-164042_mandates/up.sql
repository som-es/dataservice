-- Your SQL goes here
create table mandates (
    id serial primary key,
    delegate_id int not null,
    is_nr boolean default false,
    is_gov_official boolean default false,
    function varchar(255),
    is_ministry boolean default false,
    is_chancellor boolean default false,
    ministry varchar(312),
    name varchar(312) not null,
    party varchar(100),
    start_date date not null,
    end_date date,
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    UNIQUE (delegate_id, name, start_date)
);