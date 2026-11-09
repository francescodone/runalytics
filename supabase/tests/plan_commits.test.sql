begin;
create extension if not exists pgtap with schema extensions;
select plan(12);

insert into auth.users (id) values ('20000000-0000-0000-0000-000000000001');

select ok(not has_function_privilege('authenticated', 'public.commit_plan_head(uuid,bigint,uuid,text,integer,bytea,integer)', 'execute'), 'users cannot invoke privileged commits');
select ok(not has_function_privilege('anon', 'public.commit_plan_head(uuid,bigint,uuid,text,integer,bytea,integer)', 'execute'), 'anonymous callers cannot invoke privileged commits');
select ok(has_function_privilege('service_role', 'public.commit_plan_head(uuid,bigint,uuid,text,integer,bytea,integer)', 'execute'), 'backend can invoke privileged commits');

set local role service_role;
select results_eq(
    $$select * from public.commit_plan_head('20000000-0000-0000-0000-000000000001', 0, '30000000-0000-0000-0000-000000000001', 'plan', 1, '\x01', 1)$$,
    $$values ('committed'::text, 1::bigint)$$, 'initial commit succeeds');
select results_eq(
    $$select * from public.commit_plan_head('20000000-0000-0000-0000-000000000001', 0, '30000000-0000-0000-0000-000000000001', 'plan', 1, '\x01', 1)$$,
    $$values ('replayed'::text, 1::bigint)$$, 'identical retry returns original result');
select throws_ok(
    $$select * from public.commit_plan_head('20000000-0000-0000-0000-000000000001', 0, '30000000-0000-0000-0000-000000000001', 'plan', 1, '\x02', 1)$$,
    '22023', 'Mutation ID already used', 'mutation IDs cannot be reused for other requests');
select results_eq(
    $$select * from public.commit_plan_head('20000000-0000-0000-0000-000000000001', 0, '30000000-0000-0000-0000-000000000002', 'empty', 1, '\x02', 1)$$,
    $$values ('conflict'::text, 1::bigint)$$, 'stale writes conflict');
select results_eq(
    $$select * from public.commit_plan_head('20000000-0000-0000-0000-000000000001', 1, '30000000-0000-0000-0000-000000000003', 'empty', 1, '\x03', 1)$$,
    $$values ('committed'::text, 2::bigint)$$, 'tombstone advances revision');
reset role;
select is((select state from public.plan_heads where user_id = '20000000-0000-0000-0000-000000000001'), 'empty', 'tombstone is stored');
select is((select envelope from runalytics_private.plan_payloads where user_id = '20000000-0000-0000-0000-000000000001'), '\x03'::bytea, 'payload and head change together');
select throws_ok(
    $$select * from public.commit_plan_head('20000000-0000-0000-0000-000000000001', 2, '30000000-0000-0000-0000-000000000004', 'plan', 1, ''::bytea, 1)$$,
    '22023', 'Invalid plan commit', 'empty envelopes are rejected');
select is((select version from public.plan_heads where user_id = '20000000-0000-0000-0000-000000000001'), 2::bigint, 'invalid commit leaves revision unchanged');

select * from finish();
rollback;