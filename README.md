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
