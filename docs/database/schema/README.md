# Alleycat database

## Description

Generated from migrations with tbls. Run `cargo xtask docs db` to regenerate;  
do not edit these files manually.  
  
[Complete PostgreSQL schema, including enum values and routines](../schema.sql).  
[Architecture decisions](../../adr/README.md).  


## Tables

| Name | Columns | Comment | Type |
| ---- | ------- | ------- | ---- |
| [public.players](public.players.md) | 8 |  | BASE TABLE |
| [public.device_logs](public.device_logs.md) | 10 |  | BASE TABLE |
| [public.events](public.events.md) | 6 |  | BASE TABLE |
| [public.reserved_pdn_codes](public.reserved_pdn_codes.md) | 2 |  | BASE TABLE |
| [public.app_state](public.app_state.md) | 2 |  | BASE TABLE |
| [public.teams](public.teams.md) | 3 |  | BASE TABLE |
| [public.event_scoring_periods](public.event_scoring_periods.md) | 4 | Event scoring history. Opening creates a row; closing fills closes_at. Match eligibility uses device occurrence time, not upload time. No unclosed period means scoring is disabled. | BASE TABLE |
| [public.quickdraw_matches](public.quickdraw_matches.md) | 12 | Completed two-player quickdraw results, including non-scoring duels. Ingestion must use INSERT ... ON CONFLICT (id) DO NOTHING: the first accepted report and its device timestamp win. No per-device reports or leaderboard totals are stored. | BASE TABLE |
| [public.affiliation_categories](public.affiliation_categories.md) | 4 | Operator-defined affiliation categories within an event. No seeded categories or implied gameplay behavior. | BASE TABLE |
| [public.affiliations](public.affiliations.md) | 4 | Operator-defined affiliations within a category. Names have no implicit gameplay or permission behavior. | BASE TABLE |
| [public.player_affiliations](public.player_affiliations.md) | 4 | At most one affiliation per player per category. Composite foreign keys keep player, category, and affiliation in the same event. Clearing an assignment deletes its row; changes do not affect scores. | BASE TABLE |

## Stored procedures and functions

| Name | ReturnType | Arguments | Type |
| ---- | ------- | ------- | ---- |
| public.event_uuid_range | event_uuid_range | uuid, uuid | FUNCTION |
| public.event_uuid_range | event_uuid_range | uuid, uuid, text | FUNCTION |
| public.event_uuid_multirange | event_uuid_multirange |  | FUNCTION |
| public.event_uuid_multirange | event_uuid_multirange | event_uuid_range | FUNCTION |
| public.event_uuid_multirange | event_uuid_multirange | VARIADIC event_uuid_range[] | FUNCTION |

## Enums

| Name | Values |
| ---- | ------- |
| public.quickdraw_scoring_status | delayed_scores_disabled, same_team, scoring, scoring_disabled, unassigned_team |

## Relations

```mermaid
erDiagram

"public.players" }o--|| "public.events" : "FOREIGN KEY (event_id) REFERENCES events(id)"
"public.players" }o--|| "public.teams" : "FOREIGN KEY (event_id, team_id) REFERENCES teams(event_id, id)"
"public.app_state" }o--o| "public.events" : "FOREIGN KEY (active_event_id) REFERENCES events(id)"
"public.teams" }o--|| "public.events" : "FOREIGN KEY (event_id) REFERENCES events(id)"
"public.event_scoring_periods" }o--|| "public.events" : "FOREIGN KEY (event_id) REFERENCES events(id)"
"public.quickdraw_matches" }o--|| "public.events" : "FOREIGN KEY (event_id) REFERENCES events(id)"
"public.quickdraw_matches" }o--|| "public.teams" : "FOREIGN KEY (event_id, player_a_team_id) REFERENCES teams(event_id, id)"
"public.quickdraw_matches" }o--|| "public.teams" : "FOREIGN KEY (event_id, player_b_team_id) REFERENCES teams(event_id, id)"
"public.quickdraw_matches" }o--|| "public.players" : "FOREIGN KEY (event_id, player_a_id) REFERENCES players(event_id, id)"
"public.quickdraw_matches" }o--|| "public.players" : "FOREIGN KEY (event_id, player_b_id) REFERENCES players(event_id, id)"
"public.affiliation_categories" }o--|| "public.events" : "FOREIGN KEY (event_id) REFERENCES events(id)"
"public.affiliations" }o--|| "public.affiliation_categories" : "FOREIGN KEY (event_id, category_id) REFERENCES affiliation_categories(event_id, id)"
"public.player_affiliations" }o--|| "public.players" : "FOREIGN KEY (event_id, player_id) REFERENCES players(event_id, id)"
"public.player_affiliations" }o--|| "public.affiliations" : "FOREIGN KEY (event_id, category_id, affiliation_id) REFERENCES affiliations(event_id, category_id, id)"

"public.players" {
  uuid id
  text pdn_code
  text name
  text email
  timestamp_with_time_zone created_at
  uuid event_id FK
  uuid team_id FK
  boolean active
}
"public.device_logs" {
  uuid id
  macaddr device_mac
  bigint crash_number
  bigint uptime_ms
  timestamp_with_time_zone received_at
  text software_version
  integer reset_reason
  bigint program_counter
  bigint exception_cause
  text task_name
}
"public.events" {
  uuid id
  text name
  text venue_name
  integer next_pdn_code
  timestamp_with_time_zone created_at
  boolean accepting_delayed_scores
}
"public.reserved_pdn_codes" {
  text code
  text reason
}
"public.app_state" {
  integer id
  uuid active_event_id FK
}
"public.teams" {
  uuid id
  uuid event_id FK
  text name
}
"public.event_scoring_periods" {
  uuid id
  uuid event_id FK
  timestamp_with_time_zone started_at
  timestamp_with_time_zone closes_at
}
"public.quickdraw_matches" {
  uuid id
  uuid event_id FK
  uuid player_a_id FK
  uuid player_b_id FK
  uuid player_a_team_id FK
  uuid player_b_team_id FK
  bigint player_a_draw_time_ms
  bigint player_b_draw_time_ms
  uuid winner_player_id
  timestamp_with_time_zone occurred_at
  timestamp_with_time_zone received_at
  quickdraw_scoring_status scoring_status
}
"public.affiliation_categories" {
  uuid id
  uuid event_id FK
  text name
  boolean send_to_devices
}
"public.affiliations" {
  uuid id
  uuid event_id FK
  uuid category_id FK
  text name
}
"public.player_affiliations" {
  uuid event_id FK
  uuid player_id FK
  uuid category_id FK
  uuid affiliation_id FK
}
```

---

> Generated by [tbls](https://github.com/k1LoW/tbls)
