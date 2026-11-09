begin;
create extension if not exists pgtap with schema extensions;
select plan(5);

select is((select public from storage.buckets where id = 'runalytics-backups'), false, 'backup bucket is private');
select is((select file_size_limit from storage.buckets where id = 'runalytics-backups'), 10485760::bigint, 'backup file size is bounded');

insert into storage.objects (bucket_id, name) values ('runalytics-backups', 'test/encrypted');
set local role authenticated;
select set_config('request.jwt.claim.sub', '10000000-0000-0000-0000-000000000001', true);
select is((select count(*) from storage.objects where bucket_id = 'runalytics-backups'), 0::bigint, 'authenticated users cannot read objects directly');
select throws_ok(
    $$insert into storage.objects (bucket_id, name) values ('runalytics-backups', 'test/other')$$,
    '42501', null, 'authenticated users cannot upload objects directly');
reset role;
set local role anon;
select is((select count(*) from storage.objects where bucket_id = 'runalytics-backups'), 0::bigint, 'anonymous callers cannot read objects directly');
reset role;

select * from finish();
rollback;