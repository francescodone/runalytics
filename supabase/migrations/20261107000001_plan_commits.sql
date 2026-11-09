create table runalytics_private.plan_mutation_receipts (
    user_id uuid not null references auth.users(id) on delete cascade,
    mutation_id uuid not null,
    request_hash bytea not null,
    version bigint not null check (version > 0),
    created_at timestamptz not null default now(),
    primary key (user_id, mutation_id)
);

alter table runalytics_private.plan_mutation_receipts enable row level security;
revoke all on runalytics_private.plan_mutation_receipts from public, anon, authenticated;

create function public.commit_plan_head(
    account_id uuid,
    base_version bigint,
    mutation_id uuid,
    plan_state text,
    plan_wire_version integer,
    encrypted_envelope bytea,
    encryption_key_version integer
) returns table (result text, current_version bigint)
language plpgsql
security definer
set search_path = ''
as $$
declare
    existing_version bigint;
    receipt runalytics_private.plan_mutation_receipts%rowtype;
    payload_hash bytea;
begin
    if account_id is null or mutation_id is null or base_version is null or base_version < 0
        or plan_state is null or plan_state not in ('plan', 'empty')
        or plan_wire_version is null or plan_wire_version <= 0
        or encryption_key_version is null or encryption_key_version <= 0
        or encrypted_envelope is null or octet_length(encrypted_envelope) not between 1 and 4194304 then
        raise exception using errcode = '22023', message = 'Invalid plan commit';
    end if;

    payload_hash := extensions.digest(
        convert_to(jsonb_build_array(base_version, plan_state, plan_wire_version,
            encryption_key_version, encode(encrypted_envelope, 'hex'))::text, 'UTF8'), 'sha256');
    perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(account_id::text, 0));

    select * into receipt from runalytics_private.plan_mutation_receipts r
        where r.user_id = account_id and r.mutation_id = commit_plan_head.mutation_id;
    if found then
        if receipt.request_hash <> payload_hash then
            raise exception using errcode = '22023', message = 'Mutation ID already used';
        end if;
        return query select 'replayed'::text, receipt.version;
        return;
    end if;

    select h.version into existing_version from public.plan_heads h where h.user_id = account_id;
    existing_version := coalesce(existing_version, 0);
    if existing_version <> base_version then
        return query select 'conflict'::text, existing_version;
        return;
    end if;

    insert into public.plan_heads as h (user_id, version, state, wire_version)
        values (account_id, existing_version + 1, plan_state, plan_wire_version)
        on conflict (user_id) do update set version = excluded.version,
            state = excluded.state, wire_version = excluded.wire_version, updated_at = now();
    insert into runalytics_private.plan_payloads as p (user_id, envelope, key_version)
        values (account_id, encrypted_envelope, encryption_key_version)
        on conflict (user_id) do update set envelope = excluded.envelope,
            key_version = excluded.key_version, updated_at = now();
    insert into runalytics_private.plan_mutation_receipts (user_id, mutation_id, request_hash, version)
        values (account_id, mutation_id, payload_hash, existing_version + 1);
    return query select 'committed'::text, existing_version + 1;
end;
$$;

revoke all on function public.commit_plan_head(uuid, bigint, uuid, text, integer, bytea, integer)
    from public, anon, authenticated;
grant execute on function public.commit_plan_head(uuid, bigint, uuid, text, integer, bytea, integer)
    to service_role;