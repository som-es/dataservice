# dataservice

How to run

- Install libpq
macos
```
brew install libpq
```

Connect to postgres
```
psql dataservice
```

Look at the data:

```sql
-- call to orders per delegate 
select receiver_id,COUNT(*) from call_to_order group by call_to_order.receiver_id;

--  get the top delegates by seconds spoken, also get the delegate name due to joining
select delegates.name, delegates.party, SUM(plenar_speeches.duration_in_seconds) AS seconds from plenar_speeches inner join delegates on delegates.id = plenar_speeches.delegate_id group by plenar_speeches.delegate_id, delegates.name, delegates.party where delegates.council = 'nr' order by seconds DESC;

select delegates.name, delegates.party, SUM(plenar_speeches.duration_in_seconds) / (60.0*60.0) AS hours_spoken from plenar_speeches inner join delegates on delegates.id = plenar_speeches.delegate_id group by plenar_speeches.delegate_id, delegates.name, delegates.party,delegates.council having delegates.council = 'nr' order by hours_spoken DESC;
```

where delegates.council is 'nr'