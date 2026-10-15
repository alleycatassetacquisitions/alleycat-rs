use crate::project::project_root;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command as Process, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Subcommand)]
pub enum Command {
    /// Export schema.sql and tbls Markdown using a disposable PostgreSQL 18 container.
    Db {
        /// Output directory, relative to the repository root unless absolute.
        #[arg(long, default_value = "docs/database")]
        output: PathBuf,
        /// Fail if generated files differ from the output directory; do not write files.
        #[arg(long)]
        check: bool,
        /// Verify the reset inventory in a second, freshly created container database.
        #[arg(long)]
        check_reset: bool,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Db {
            output,
            check,
            check_reset,
        } => generate(output, check, check_reset),
    }
}

// Drop cleans up after both success and ordinary error returns. No ports or volumes
// are published, and this container never uses the application's credentials.
struct Database(String);
impl Drop for Database {
    fn drop(&mut self) {
        if !Process::new("docker")
            .args(["rm", "-f", &self.0])
            .output()
            .is_ok_and(|o| o.status.success())
        {
            eprintln!(
                "Could not remove documentation container {}; remove it with docker rm -f {}",
                self.0, self.0
            );
        }
    }
}

fn docker(args: &[&str], input: Option<&[u8]>) -> Result<String> {
    let mut child = Process::new("docker")
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("could not start Docker; install Docker and start its daemon")?;
    let write_result = if let Some(input) = input {
        child
            .stdin
            .take()
            .context("missing Docker stdin")?
            .write_all(input)
    } else {
        Ok(())
    };
    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!("Docker command failed with {}", output.status);
    }
    write_result?;
    Ok(String::from_utf8(output.stdout)?)
}

fn generate(output: PathBuf, check: bool, check_reset: bool) -> Result<()> {
    let root = project_root()?;
    let output = root.join(output);
    let name = format!(
        "alleycat-docs-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    println!("Starting temporary PostgreSQL 18 for database documentation...");
    docker(
        &[
            "run",
            "--detach",
            "--rm",
            "--name",
            &name,
            "--network",
            "none",
            "--env",
            "POSTGRES_HOST_AUTH_METHOD=trust",
            "--env",
            "POSTGRES_DB=docs",
            "--tmpfs",
            "/var/lib/postgresql",
            "postgres:18",
        ],
        None,
    )?;
    let db = Database(name);
    let start = Instant::now();
    loop {
        if Process::new("docker")
            // The image's bootstrap server only listens on a Unix socket.
            // Wait for TCP so migrations cannot race its final restart.
            .args([
                "exec",
                &db.0,
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                "docs",
            ])
            .output()?
            .status
            .success()
        {
            break;
        }
        if start.elapsed() > Duration::from_secs(60) {
            bail!("temporary PostgreSQL did not become ready in 60 seconds");
        }
        thread::sleep(Duration::from_millis(250));
    }
    let mut migrations = fs::read_dir(root.join("migrations"))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    migrations.retain(|p| {
        p.extension().is_some_and(|e| e == "sql") && !p.to_string_lossy().ends_with(".down.sql")
    });
    migrations.sort();
    if migrations.is_empty() {
        bail!("no SQL migrations found");
    }
    db.migrate("docs", &migrations)?;
    let schema = db.dump("docs", true)?;
    if check_reset {
        db.check_reset(&migrations)?;
    }
    let schema_path = output.join("schema.sql");
    if check {
        if fs::read(&schema_path).ok().as_deref() != Some(schema.as_bytes()) {
            bail!(
                "Database documentation is stale or missing: {}. Run cargo xtask docs db (with the same --output, if specified) and commit the generated files.",
                schema_path.display()
            );
        }
    } else {
        fs::create_dir_all(&output)?;
        fs::write(&schema_path, schema)?;
        println!("Generated {}", schema_path.display());
    }
    db.document(&output, check)?;
    if check {
        println!("Database documentation is up to date.");
    }
    Ok(())
}

impl Database {
    fn document(&self, output: &Path, check: bool) -> Result<()> {
        let output = output
            .canonicalize()
            .context("database documentation directory is missing")?;
        let config = project_root()?.join(".tbls.yml");
        let network = format!("container:{}", self.0);
        let config_mount = format!("type=bind,src={},dst=/tbls.yml,readonly", config.display());
        let output_mount = format!(
            "type=bind,src={},dst=/docs{}",
            output.display(),
            if check { ",readonly" } else { "" }
        );
        // Join only the disposable database's isolated network namespace.
        // The schema/ subdirectory is exclusively tbls-generated: --rm-dist
        // removes obsolete table pages without touching the sibling SQL dump.
        let mut command = Process::new("docker");
        command.args([
            "run",
            "--rm",
            "--network",
            &network,
            "--mount",
            &config_mount,
            "--mount",
            &output_mount,
            "ghcr.io/k1low/tbls:v1.96.1",
            if check { "diff" } else { "doc" },
            "--config",
            "/tbls.yml",
            "postgres://postgres@127.0.0.1:5432/docs?sslmode=disable",
            "/docs/schema",
        ]);
        if !check {
            command.args(["--force", "--rm-dist"]);
        }
        let status = command.status().context("could not run tbls in Docker")?;
        if !status.success() {
            bail!(
                "tbls failed with {status}; run cargo xtask docs db to regenerate stale documentation"
            );
        }
        Ok(())
    }

    fn sql(&self, database: &str, sql: &[u8]) -> Result<String> {
        docker(
            &[
                "exec",
                "-i",
                &self.0,
                "psql",
                "-X",
                "-q",
                "-A",
                "-t",
                "-v",
                "ON_ERROR_STOP=1",
                "-U",
                "postgres",
                "-d",
                database,
            ],
            Some(sql),
        )
    }

    fn migrate(&self, database: &str, migrations: &[PathBuf]) -> Result<()> {
        for migration in migrations {
            println!(
                "Applying {} to {database}",
                migration.file_name().unwrap().to_string_lossy()
            );
            self.sql(database, &fs::read(migration)?)
                .with_context(|| format!("failed to apply {}", migration.display()))?;
        }
        Ok(())
    }

    fn dump(&self, database: &str, schema_only: bool) -> Result<String> {
        let mut args = vec!["exec", &self.0, "pg_dump", "-U", "postgres", "-d", database];
        if schema_only {
            args.extend(["--schema-only", "--no-owner", "--no-privileges"]);
        }
        Ok(normalize_dump(&docker(&args, None)?))
    }

    fn check_reset(&self, migrations: &[PathBuf]) -> Result<()> {
        // This database can only be created in our new, network-isolated container.
        // Never accept a URL or read application/cloud database configuration here.
        self.sql("docs", b"CREATE DATABASE reset_check")?;
        self.sql("reset_check", include_bytes!("reset_sentinels.sql"))?;
        // Full dump includes unrelated data, sequence values, ownership and grants,
        // as well as every schema object. No handwritten application inventory.
        let baseline = self.dump("reset_check", false)?;
        self.migrate("reset_check", migrations)?;
        let migrated = self.dump("reset_check", true)?;
        // The docs runner applies SQL directly, so create a ledger sentinel. Its
        // columns/data are immaterial: reset must drop the entire SQLx table.
        self.sql("reset_check", b"CREATE TABLE public._sqlx_migrations (version bigint PRIMARY KEY); INSERT INTO public._sqlx_migrations VALUES (1);")?;
        self.sql("reset_check", include_bytes!("../../reset_remote_db.sql"))?;
        if self.dump("reset_check", false)? != baseline {
            bail!(
                "Reset inventory check failed: application objects or the SQLx ledger remain, or unrelated objects/data were changed. Review xtask/reset_remote_db.sql."
            );
        }
        self.migrate("reset_check", migrations)?;
        if self.dump("reset_check", true)? != migrated {
            bail!("Migrations after reset did not reproduce the original database.");
        }
        println!(
            "Reset inventory removes application objects and the ledger, preserves unrelated objects/data, and allows migrations to reapply."
        );
        Ok(())
    }
}

fn normalize_dump(input: &str) -> String {
    input
        .lines()
        .filter(|line| {
            !line.starts_with("\\restrict ")
                && !line.starts_with("\\unrestrict ")
                && !line.starts_with("-- Dumped from database version")
                && !line.starts_with("-- Dumped by pg_dump version")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
