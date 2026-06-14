create type player_mode as enum ('unassigned','hunter', 'bounty');

create type player_role as enum ('staff', 'courier', 'miniboss');

create table available_pdn_codes (
    code text primary key check (code ~ '^[0-9]{4}$')
);

insert into available_pdn_codes (code)
select lpad(n::text, 4, '0')
from generate_series(0, 9999) as n
where lpad(n::text, 4, '0') not in (
    '0000',
    '1111',
    '2222',
    '3333',
    '4444',
    '5555',
    '6666',
    '7777',
    '8888',
    '9999',
    '6969',
    '8008',
    '1337'
);

create table players (
    id uuid not null,
    PRIMARY KEY (id),
    pdn_code text not null unique check (pdn_code ~ '^[0-9]{4}$'),
    name text not null unique,
    mode player_mode not null default 'unassigned',
    created_at timestamptz not null
);

create table player_roles (
  player_id uuid not null references players(id) on delete cascade,
  role player_role not null,
  primary key (player_id, role)
);
