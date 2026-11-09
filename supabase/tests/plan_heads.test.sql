begin;

create extension if not exists pgtap with schema extensions;
select plan(10);

select has_table('public', 'plan_heads', 'plan metadata exists');
select has_table('runalytics_private', 'plan_payloads', 'encrypted payload table exists');
select is((select relrowsecurity from pg_class where oid = 'public.plan_heads'::regclass), true, 'metadata enforces RLS');
select is((select relrowsecurity from pg_class where oid = 'runalytics_private.plan_payloads'::regclass), true, 'payloads enforce RLS');
select ok(not has_schema_privilege('authenticated', 'runalytics_private', 'usage'), 'clients cannot use private schema');
select ok(not has_table_privilege('authenticated', 'runalytics_private.plan_payloads', 'select'), 'clients cannot read payloads directly');
select ok(not has_table_privilege('authenticated', 'public.plan_heads', 'insert'), 'clients cannot create heads directly');
select ok(not has_table_privilege('authenticated', 'public.plan_heads', 'update'), 'clients cannot update heads directly');
select ok(not has_table_privilege('authenticated', 'public.plan_heads', 'delete'), 'clients cannot delete heads directly');

insert into auth.users (id) values
    ('10000000-0000-0000-0000-000000000001'),
    ('10000000-0000-0000-0000-000000000002');
insert into public.plan_heads (user_id, version, state, wire_version) values
    ('10000000-0000-0000-0000-000000000001', 1, 'plan', 1),
    ('10000000-0000-0000-0000-000000000002', 1, 'empty', 1);

set local role authenticated;
select set_config('request.jwt.claim.sub', '10000000-0000-0000-0000-000000000001', true);
select results_eq(
    'select user_id from public.plan_heads',
    $$values ('10000000-0000-0000-0000-000000000001'::uuid)$$,
    'a user sees only their own plan metadata'
);
reset role;

select * from finish();
rollback;