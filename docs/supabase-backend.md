# Supabase backend foundation

This is an initial, inactive database foundation. Runalytics does not yet
connect to Supabase, create accounts, synchronize plans, or broker Strava
authorization. Existing local storage, Gist synchronization, and provider
connections are unchanged. Do not deploy this foundation or store user data
until the authentication, encryption, retention, and recovery work is complete.

## Plan storage boundary

`public.plan_heads` contains account-owned plan metadata. Authenticated users
can read only their own row and cannot directly create, change, or delete rows.
The associated `runalytics_private.plan_payloads` table is not accessible to
client roles. It is reserved for encrypted envelopes, not plaintext plans.
Its 4 MiB limit matches the existing Gist sync response bound.

`commit_plan_head` is a backend-only database function. It serializes commits
per account, compares the expected revision, and stores a receipt so an
identical retry returns the original revision. A reused mutation ID with a
different request is rejected. Only the Supabase service role may call it.
The future Edge Function must derive the account ID from a verified user
session, never from the request body; the database function itself trusts its
privileged caller and is not a user-facing API.

There is no application API, encryption, or key management yet. Those are
required before any application can write or read plan payloads. Receipts
currently persist for the lifetime of the account; retention and deletion
handling must be implemented before release. The current Gist encryption
format must not be reused as the new account encryption scheme.

## Private backups

The `runalytics-backups` Storage bucket is private and limits each object to
10 MiB. There are no client access policies for it, so authenticated and
anonymous clients cannot read or upload directly. It is not an operational
backup feature: server-side encryption, account authorization, aggregate
quotas, retention, deletion, and independent disaster recovery remain to be
implemented and tested before use.

## Local verification

Install the Supabase CLI and start a working Docker daemon, then from the
repository root run:

```sh
supabase start
supabase db reset
supabase test db
```

The pgTAP tests check metadata ownership, row-level security, retry-safe plan
commits, and denial of direct client access to private data. Expand them with
transactional concurrency and two-user API tests as the backend is implemented.
Use local or synthetic data only; do not put production credentials in this
configuration.

## Release prerequisites

Before accepting real data, complete the account and consent flows, encryption
and key recovery, retention/deletion, independent backups, Strava app review,
and two-device offline and migration testing. Establish the Supabase project,
processor arrangements, and privacy disclosures with the maintainer. The free
plan is subject to service limits and does not include automatic backups.