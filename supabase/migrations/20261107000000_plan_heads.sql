create schema if not exists runalytics_private;

revoke all on schema runalytics_private from public, anon, authenticated;

create table public.plan_heads (
    user_id uuid primary key references auth.users(id) on delete cascade,
    version bigint not null check (version > 0),
    state text not null check (state in ('plan', 'empty')),
    wire_version integer not null check (wire_version > 0),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

alter table public.plan_heads enable row level security;

revoke all on public.plan_heads from public, anon, authenticated;
grant select on public.plan_heads to authenticated;

create policy plan_heads_select_own on public.plan_heads
    for select to authenticated
    using ((select auth.uid()) = user_id);

create table runalytics_private.plan_payloads (
    user_id uuid primary key references public.plan_heads(user_id) on delete cascade,
    envelope bytea not null check (octet_length(envelope) between 1 and 4194304),
    key_version integer not null check (key_version > 0),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

alter table runalytics_private.plan_payloads enable row level security;
revoke all on runalytics_private.plan_payloads from public, anon, authenticated;