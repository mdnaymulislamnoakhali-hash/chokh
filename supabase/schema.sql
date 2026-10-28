-- =====================================================================
-- Chokh - চোখ | Supabase setup script (Phase 1)
-- Supabase Dashboard → SQL Editor-এ পুরোটা পেস্ট করে Run করুন।
-- বারবার চালালেও সমস্যা নেই (idempotent)।
-- =====================================================================

-- 1) ইভেন্ট টেবিল -----------------------------------------------------
create table if not exists public.analytics (
  id          bigint generated always as identity primary key,
  event_id    uuid        not null unique,      -- অফলাইন কিউ আবার পাঠালে ডুপ্লিকেট ঠেকায়
  user_id     uuid        not null,             -- বেনামী UUID (কোনো ব্যক্তিগত তথ্য নয়)
  event       text        not null
              check (event in ('install','app_open','break_taken','break_skipped')),
  event_ts    timestamptz not null,             -- ক্লায়েন্টে ঘটনা ঘটার সময়
  app_version text        not null default 'unknown',
  os_type     text        not null default 'unknown',
  created_at  timestamptz not null default now() -- সার্ভারে পৌঁছানোর সময়
);

create index if not exists analytics_user_idx     on public.analytics (user_id, event_ts desc);
create index if not exists analytics_event_ts_idx on public.analytics (event, event_ts desc);
create index if not exists analytics_ts_idx       on public.analytics (event_ts desc);

-- RLS চালু + কোনো policy নেই = anon/authenticated কেউ সরাসরি পড়তে/লিখতে পারবে না।
-- শুধু Edge Function (service role) লিখতে পারবে, আর অ্যাডমিন পড়বে নিচের RPC দিয়ে।
alter table public.analytics enable row level security;
revoke all on public.analytics from anon, authenticated;

-- 2) অ্যাডমিন তালিকা ----------------------------------------------------
create table if not exists public.admins (
  email text primary key
);
alter table public.admins enable row level security;
revoke all on public.admins from anon, authenticated;

-- ⚠️ নিচের ইমেইল বদলে আপনার অ্যাডমিন ইমেইল দিন (Supabase Auth-এ এই ইমেইলে ইউজার বানাবেন)
insert into public.admins (email) values ('YOUR_ADMIN_EMAIL@example.com')
on conflict do nothing;

create or replace function public.is_admin()
returns boolean
language sql stable security definer
set search_path = public
as $$
  select exists (
    select 1 from public.admins
    where lower(email) = lower(coalesce(auth.jwt() ->> 'email', ''))
  );
$$;

-- 3) ড্যাশবোর্ডের জন্য RPC ফাংশন (দিনের সীমা: Asia/Dhaka) -------------------

-- 3a) সারাংশ: মোট ইনস্টল, DAU/WAU/MAU, রেস্ট বনাম স্কিপ
create or replace function public.admin_summary()
returns json
language plpgsql stable security definer
set search_path = public
as $$
declare
  tz          text        := 'Asia/Dhaka';
  today_start timestamptz := date_trunc('day', now() at time zone tz) at time zone tz;
begin
  if not public.is_admin() then
    raise exception 'forbidden' using errcode = '42501';
  end if;

  return json_build_object(
    'total_installs', (select count(distinct user_id) from public.analytics),
    'dau', (select count(distinct user_id) from public.analytics
              where event_ts >= today_start),
    'wau', (select count(distinct user_id) from public.analytics
              where event_ts >= today_start - interval '6 days'),
    'mau', (select count(distinct user_id) from public.analytics
              where event_ts >= today_start - interval '29 days'),
    'breaks_taken',   (select count(*) from public.analytics where event = 'break_taken'),
    'breaks_skipped', (select count(*) from public.analytics where event = 'break_skipped')
  );
end;
$$;

-- 3b) গত ৩০ দিনের নতুন ইউজার (যেদিন প্রথম দেখা গেছে সেদিনই ইনস্টল ধরা হয়)
create or replace function public.admin_installs_30d()
returns table (day_date date, install_count bigint)
language plpgsql stable security definer
set search_path = public
as $$
declare
  tz text := 'Asia/Dhaka';
begin
  if not public.is_admin() then
    raise exception 'forbidden' using errcode = '42501';
  end if;

  return query
  with days as (
    select d::date as d
    from generate_series(
           (now() at time zone tz)::date - 29,
           (now() at time zone tz)::date,
           interval '1 day') as d
  ),
  first_seen as (
    select a.user_id, min(a.event_ts) as ts
    from public.analytics a
    group by a.user_id
  )
  select days.d, count(fs.user_id)
  from days
  left join first_seen fs on (fs.ts at time zone tz)::date = days.d
  group by days.d
  order by days.d;
end;
$$;

-- 3c) সাম্প্রতিক ইউজারের তালিকা
create or replace function public.admin_recent_users(p_limit int default 50)
returns table (
  u_user_id uuid, u_first_seen timestamptz, u_last_seen timestamptz,
  u_events bigint, u_app_version text, u_os_type text
)
language plpgsql stable security definer
set search_path = public
as $$
begin
  if not public.is_admin() then
    raise exception 'forbidden' using errcode = '42501';
  end if;

  return query
  select a.user_id,
         min(a.event_ts),
         max(a.event_ts),
         count(*),
         (array_agg(a.app_version order by a.event_ts desc))[1],
         (array_agg(a.os_type     order by a.event_ts desc))[1]
  from public.analytics a
  group by a.user_id
  order by max(a.event_ts) desc
  limit least(greatest(p_limit, 1), 200);
end;
$$;

-- শুধু লগইন করা ইউজার ফাংশন কল করতে পারবে (ভেতরে is_admin() আবার যাচাই করে)
revoke all on function public.is_admin()                  from public, anon;
revoke all on function public.admin_summary()             from public, anon;
revoke all on function public.admin_installs_30d()        from public, anon;
revoke all on function public.admin_recent_users(int)     from public, anon;
grant execute on function public.is_admin()               to authenticated;
grant execute on function public.admin_summary()          to authenticated;
grant execute on function public.admin_installs_30d()     to authenticated;
grant execute on function public.admin_recent_users(int)  to authenticated;
