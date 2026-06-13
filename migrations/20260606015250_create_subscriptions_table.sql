create type player_mode as enum ('unassigned','hunter', 'bounty');

create type player_role as enum ('staff', 'courier', 'miniboss');

create table players (
    id uuid not null,
    PRIMARY KEY (id),
    name text not null unique,
    mode player_mode not null default 'unassigned',
    created_at timestamptz not null
);

create table player_roles (
  player_id uuid not null references players(id) on delete cascade,
  role player_role not null,
  primary key (player_id, role)
);
