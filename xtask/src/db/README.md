# Reset verification

With Docker running, `cargo xtask db check-reset` starts an isolated PostgreSQL 18 container,
applies migrations, exercises `xtask/reset_remote_db.sql`, and checks that only
application objects and a SQLx ledger sentinel disappear. Unrelated objects,
data, and grants must survive, and migrations must apply again to produce the
same database. Migration SQL is applied directly; the ledger sentinel tests
table removal, not SQLx migration bookkeeping.
