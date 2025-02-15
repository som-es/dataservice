-- Your SQL goes here
create table plenar_speech_legis_inits(
    id serial primary key,
    gp varchar(255) not null,
    ityp varchar(255) not null,
    inr int not null,
    speech_id int not null,
    legis_init_id int, -- optional reference
    foreign key (legis_init_id) references legislative_initiatives(id),
    foreign key (speech_id) references plenar_speeches(id),
    unique(gp, ityp, inr, speech_id)
);

create table legis_inits_refs(
    id serial primary key,
    origin_legis_init_id int not null,
    ref_gp varchar(255) not null,
    ref_ityp varchar(255) not null,
    ref_inr int not null,
    foreign key (origin_legis_init_id) references legislative_initiatives(id),
    unique(ref_gp, ref_ityp, ref_inr, origin_legis_init_id)
);
