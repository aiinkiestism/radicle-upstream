use std::io::Write;
use std::path::Path;
use std::{fs, fs::File, os::unix::fs::PermissionsExt};

use librad::SecretKey;

fn main() {
    init_logging();

    let args: upstream_seed::cli::Args = structopt::StructOpt::from_args();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let key_path = args.rad_home.join("identity.key");
    if key_path.exists() {
        if let Err(e) = generate_identity(&key_path) {
            tracing::error!(target: "org-node", "Fatal: error creating identity: {:#}", e);
            std::process::exit(2);
        }
        tracing::info!(target: "org-node", "Identity file generated: {:?}", key_path);
    }

    if let Err(e) = upstream_seed::run(
        rt,
        upstream_seed::Options {
            rad_home: args.rad_home,
            bootstrap: args.bootstrap.unwrap_or_default(),
            listen: args.listen,
            projects: args.project,
            key_path,
        },
    ) {
        tracing::error!(target: "org-node", "Fatal: {:#}", e);
        std::process::exit(1);
    }
}

fn generate_identity(path: &Path) -> anyhow::Result<()> {
    let mut file = File::create(path)?;
    let metadata = file.metadata()?;
    let mut permissions = metadata.permissions();

    permissions.set_mode(0o600);
    fs::set_permissions(path, permissions)?;

    let secret_key = SecretKey::new();
    file.write_all(secret_key.as_ref())?;

    Ok(())
}

fn init_logging() {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "full");
    }

    let env_filter = if let Ok(value) = std::env::var("RUST_LOG") {
        tracing_subscriber::EnvFilter::new(value)
    } else {
        let directives = [
            "info",
            "quinn=warn",
            "api=debug",
            "radicle_daemon=debug",
            "librad=debug",
            // Silence some noisy debug statements
            "librad::git::refs=info",
            "librad::git::include=info",
            "librad::git::identities::person=info",
            "librad::git::identities::local=info",
            "librad::net::protocol::membership::periodic=info",
            "librad::git::tracking=info",
        ];

        let mut env_filter = tracing_subscriber::EnvFilter::default();

        for directive in directives {
            env_filter = env_filter.add_directive(directive.parse().expect("invalid log directive"))
        }
        env_filter
    };

    let builder = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(env_filter);

    match std::env::var("TRACING_FMT").as_deref() {
        Ok("pretty") => builder.pretty().init(),
        Ok("compact") => builder.compact().init(),
        Ok("json") => builder.json().init(),
        _ => builder.pretty().init(),
    };
}
