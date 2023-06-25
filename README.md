# dataservice

How to run

- Install libpq
macos
```
brew install libpq
```

# Debian Setup

```
https://www.postgresql.org/download/linux/debian/
```

sudo -i
#### login as postgres user
sudo -i -u postgres

createdb dataservice

Connect to postgres
```
psql dataservice
```

Update the password for the postgres user.

```
alter role postgres password '123';
```

change the database (username, password) in the .env
```
DATABASE_URL=postgres://postgres:123@localhost/dataservice
```

Look at the data:

```sql
-- call to orders per delegate 
select receiver_id,COUNT(*) from call_to_order group by call_to_order.receiver_id;

--  get the top delegates by seconds spoken, also get the delegate name due to joining
select delegates.name, delegates.party, SUM(plenar_speeches.duration_in_seconds) / (60.0*60.0) AS hours_spoken from plenar_speeches inner join delegates on delegates.id = plenar_speeches.delegate_id group by plenar_speeches.delegate_id, delegates.name, delegates.party,delegates.council having delegates.council = 'nr' order by hours_spoken DESC;

-- ... but only from the given legislative period (only that)
select plenar_speeches.debate_id, plenar_speeches.delegate_id from plenar_speeches inner join debates on plenar_speeches.debate_id = debates.id inner join plenar_infos on debates.plenar_id = plenar_infos.id where plenar_infos.legislative_period = 'XXVII';

-- improve performance by adding an index?
create index on plenar_infos (legislative_period);

-- MIND: the resulting hours are not relative to the total tenure (amtzeit)
select 
  delegates.name, 
  delegates.party, 
  SUM(
    plenar_speeches.duration_in_seconds
  ) / (60.0 * 60.0) AS hours_spoken 
from 
  plenar_speeches 
  inner join debates on plenar_speeches.debate_id = debates.id 
  inner join plenar_infos on debates.plenar_id = plenar_infos.id 
  inner join delegates on delegates.id = plenar_speeches.delegate_id 
where 
  plenar_infos.legislative_period = 'XXVII' 
  and delegates.council = 'nr'
group by 
  plenar_speeches.delegate_id, 
  delegates.name, 
  delegates.party, 
  delegates.council 
-- having 
--   delegates.council = 'nr' 
order by 
  hours_spoken DESC;

-- get delegates
select name from delegates where is_active='t' and council='nr' and seat_row is not null;


```


```sql

-- received call to orders for delegates in party

select 
  delegates.party, 
  COUNT(*)::Integer as call_to_order_amount, 
  delegates_in_party.delegate_amount, 
  (
    COUNT(*):: real / delegates_in_party.delegate_amount
  )::real as ratio
from 
  call_to_order 
  inner join delegates on delegates.id = call_to_order.receiver_id 
  join (
    select 
      party, 
      COUNT(*)::Integer as delegate_amount
    from 
      delegates 
    where 
      is_active = 't' 
      and council = 'nr' 
      and seat_row is not null 
    group by 
      party
  ) as delegates_in_party on delegates_in_party.party = delegates.party 
group by 
  delegates.party, 
  delegates_in_party.delegate_amount
order by
  ratio DESC;
```

```sql
select delegates.name,delegates.gender,plenar_speeches.duration_in_seconds from plenar_speeches inner join delegates on 
delegates.id = plenar_speeches.delegate_id where delegates.is_active='t' and delegates.council='nr' and delegates.seat_row is 
not null;

select 
  delegates.gender,
  SUM(plenar_speeches.duration_in_seconds) 
from 
  plenar_speeches 
  inner join delegates on delegates.id = plenar_speeches.delegate_id 
  inner join debates on plenar_speeches.debate_id = debates.id
  inner join plenar_infos on debates.plenar_id = plenar_infos.id
where plenar_infos.legislative_period = 'XXVII' and delegates.is_active='t' and delegates.council='nr' and delegates.seat_row is not null 
group by delegates.gender;


select 
  delegates.gender,
  SUM(plenar_speeches.duration_in_seconds) 
from 
  plenar_speeches 
  inner join delegates on delegates.id = plenar_speeches.delegate_id
  inner join mandates on mandates.delegate_id = delegates.id
  inner join debates on plenar_speeches.debate_id = debates.id
  inner join plenar_infos on debates.plenar_id = plenar_infos.id
where plenar_infos.legislative_period = 'XXVII' and mandates.name like '%Abge% %National%' 
group by delegates.gender;

select delegates.name from mandates inner join delegates on delegates.id = mandates.delegate_id where mandates.name like '%Abge% %National%';


-- looks at a specific debate -> look at opinion
select debates.about,plenar_speeches.delegate_id,plenar_speeches.opinion,plenar_speeches.spoken_text from plenar_speeches inner join debates on debates.id = plenar_speeches.debdataservice=# select debates.about,plenar_speeches.delegatdataservice=# select debates.about,plenar_speeches.delegate_id,plenar_speeches.opinion,plenar_speeches.spoken_text from plenar_speeches inner join debates on debates.id = plenar_speeches.debate_id where plenar_id=3;


select 
  delegates.gender,
  SUM(plenar_speeches.duration_in_seconds) 
from 
  plenar_speeches 
  inner join delegates on delegates.id = plenar_speeches.delegate_id
  inner join mandates on mandates.delegate_id = delegates.id
  inner join debates on plenar_speeches.debate_id = debates.id
  inner join plenar_infos on debates.plenar_id = plenar_infos.id
where plenar_infos.legislative_period = 'XXVI' and mandates.name like '%Abge% %National%' 
group by delegates.gender;

```

```sql
select 
        delegates.name,
        delegates.image_url,
        delegates.party,
        cast(COUNT(*) as Integer) as call_to_order_amount
        from call_to_order 
        inner join delegates on call_to_order.receiver_id=delegates.id 
        inner join plenar_infos on call_to_order.plenar_id = plenar_infos.id
        where plenar_infos.legislative_period = 'XXVI'
        group by delegates.name,delegates.image_url,delegates.party,call_to_order.receiver_id 
        order by call_to_order_amount DESC;

```

# other ideas

in which plenar sitzungen received someone call to orders
most proposals per delegate
most proposals per party

most proposals per party per legislative period

most speeches held per delegate (not in hours)
most speeches held per party

```sql
select delegates.name,COUNT(opinion) as opinions from plenar_speeches inner join delegates on delegates.id = plenar_speeches.delegate_id where (opinion = 'Pro' or opinion = 'Contra') and is_active='t' group by delegates.name order by opinions desc; -- and is_active='t'

select delegates.name,COUNT(opinion) as opinions from plenar_speeches inner join delegates on delegates.id = plenar_speeches.delegate_id where (opinion = 'Pro' or opinion = 'Contra') and delegates.name like '%Himmelbauer%' group by delegates.name order by opinions desc;

select delegates.name,COUNT(opinion) as opinions from plenar_speeches inner join delegates on delegates.id = plenar_speeches.delegate_id where delegates.name like '%Himmelbauer%' group by delegates.name order by opinions desc;

```