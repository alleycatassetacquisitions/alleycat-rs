use crate::project::project_root;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command as Process, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Subcommand)]
pub enum Command {
    /// Export schema.sql and README.md using a disposable PostgreSQL 18 container.
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
    let catalog = db.sql("docs", include_bytes!("catalog.sql"))?;
    let markdown = render(&serde_json::from_str(&catalog)?);
    if check_reset {
        db.check_reset(&migrations)?;
    }
    let files = [("schema.sql", schema), ("README.md", markdown)];
    if check {
        let stale: Vec<_> = files
            .iter()
            .filter(|(name, contents)| {
                fs::read(output.join(name)).ok().as_deref() != Some(contents.as_bytes())
            })
            .map(|(name, _)| output.join(name).display().to_string())
            .collect();
        if !stale.is_empty() {
            bail!(
                "Database documentation is stale or missing: {}. Run cargo xtask docs db (with the same --output, if specified) and commit the generated files.",
                stale.join(", ")
            );
        }
        println!("Database documentation is up to date.");
    } else {
        fs::create_dir_all(&output)?;
        for (name, contents) in files {
            fs::write(output.join(name), contents)?;
            println!("Generated {}", output.join(name).display());
        }
    }
    Ok(())
}

impl Database {
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
fn cell(value: &Value) -> String {
    value
        .as_str()
        .unwrap_or("")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace('\r', "")
        .replace('\n', "<br>")
}
fn diagram_label(s: &str) -> String {
    s.replace('&', "#38;")
        .replace('"', "#34;")
        .replace('<', "#60;")
        .replace('>', "#62;")
        .replace(['\n', '\r'], " ")
}
fn render(catalog: &Value) -> String {
    let tables = catalog["tables"].as_array().expect("catalog tables");
    let names: Vec<String> = tables
        .iter()
        .map(|t| {
            format!(
                "{}.{}",
                t["schema"].as_str().unwrap(),
                t["name"].as_str().unwrap()
            )
        })
        .collect();
    let mut out = String::from(
        "# Database schema\n\nGenerated from the repository’s migrations. After changing migrations, run `cargo xtask docs db` with Docker running, then commit the updated documentation. Do not edit this file manually.\n\n[View the complete SQL schema](schema.sql).\n\n## Relationships\n\nArrows point from the referencing table to the referenced table. Labels show foreign key definitions; cardinality is not inferred.\n\n```mermaid\nflowchart LR\n",
    );
    for (i, name) in names.iter().enumerate() {
        out.push_str(&format!("    t{i}[\"{}\"]\n", diagram_label(name)));
    }
    for r in catalog["relationships"].as_array().unwrap() {
        if let (Some(from), Some(to)) = (
            names
                .iter()
                .position(|n| Some(n.as_str()) == r["from"].as_str()),
            names
                .iter()
                .position(|n| Some(n.as_str()) == r["to"].as_str()),
        ) {
            out.push_str(&format!(
                "    t{from} -->|\"{}\"| t{to}\n",
                diagram_label(r["definition"].as_str().unwrap())
            ));
        }
    }
    out.push_str("```\n\n## Enum types\n\n| Type | Values |\n| --- | --- |\n");
    for e in catalog["enums"].as_array().unwrap() {
        out.push_str(&format!(
            "| {} | {} |\n",
            cell(&e["name"]),
            e["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(cell)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    for (t, name) in tables.iter().zip(names) {
        out.push_str(&format!("\n## {}\n\n{}\n\n| Column | Type | Nullable | Default | Description |\n| --- | --- | --- | --- | --- |\n", cell(&Value::String(name)), cell(&t["comment"])));
        for c in t["columns"].as_array().unwrap() {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                cell(&c["name"]),
                cell(&c["type"]),
                if c["nullable"] == true { "yes" } else { "no" },
                cell(&c["default"]),
                cell(&c["comment"])
            ));
        }
        out.push_str("\n### Constraints\n\n| Name | Definition |\n| --- | --- |\n");
        for c in t["constraints"].as_array().unwrap() {
            out.push_str(&format!(
                "| {} | {} |\n",
                cell(&c["name"]),
                cell(&c["definition"])
            ));
        }
        out.push_str("\n### Indexes\n\n| Definition |\n| --- |\n");
        for i in t["indexes"].as_array().unwrap() {
            out.push_str(&format!("| {} |\n", cell(i)));
        }
    }
    out
}
