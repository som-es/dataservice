-- Your SQL goes here
CREATE TABLE plenar_infos (
    -- use serial id -> add inr, and UNIQUE of (inr, ityp, gp)
    id serial primary key,
    add_date timestamp not null,
    inr int not null,
    title varchar(255) not null,
    description varchar(255) not null,
    legislative_period varchar(9) not null,
    UNIQUE (inr, legislative_period, add_date)
);

CREATE TABLE call_to_order (
    id varchar(20) primary key,
    plenar_id int not null,
    inr int not null,
    receiver_id int not null,
    FOREIGN KEY (plenar_id) REFERENCES plenar_infos(id),
    FOREIGN KEY (receiver_id) REFERENCES delegates(id)
);

CREATE TABLE debates (
    id serial primary key,
    -- use serial id -> ..
    plenar_id int not null,
    -- plenar_id varchar(20) not null,
    about varchar(512) not null,
    FOREIGN KEY (plenar_id) REFERENCES plenar_infos(id),
    UNIQUE (plenar_id, about)
);

CREATE TABLE plenar_speeches (
    id serial primary key,
    delegate_id int not null,
    debate_id int not null,
    opinion varchar(40),
    spoken_text text unique,
    duration_in_seconds int,
    ty varchar(40),
    delegate_display_name varchar(255) not null,
    FOREIGN KEY (debate_id) REFERENCES debates(id),
    FOREIGN KEY (delegate_id) REFERENCES delegates(id),
    UNIQUE (delegate_id, debate_id)
);

-- index is too large for speech text
alter table plenar_speeches drop constraint plenar_speeches_spoken_text_key;
