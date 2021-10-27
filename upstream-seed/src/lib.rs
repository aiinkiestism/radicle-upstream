//! # Org Node
//!
//! The purpose of the org node is to listen for on-chain anchor events and
//! start replicating the associated radicle projects.
//!
//! The org node can be configured to listen to any number of orgs, or *all*
//! orgs.
use anyhow::Context;

use librad::profile::Profile;
use thiserror::Error;

use tokio::sync::mpsc;

use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::net;
use std::path::PathBuf;

pub mod cli;
mod client;

use client::{Client, Urn};

/// Org identifier (Ethereum address).
pub type OrgId = String;

#[derive(Debug, Clone)]
pub struct Options {
    pub rad_home: PathBuf,
    pub key_path: PathBuf,
    pub bootstrap: Vec<(librad::PeerId, net::SocketAddr)>,
    pub listen: net::SocketAddr,
    pub projects: Vec<Urn>,
}

/// Error parsing a Radicle URN.
#[derive(Error, Debug)]
enum ParseUrnError {
    #[error(transparent)]
    Int(#[from] std::num::ParseIntError),
    #[error(transparent)]
    Git(#[from] git2::Error),
}

#[derive(serde::Deserialize, Debug)]
struct Anchor {
    #[serde(rename(deserialize = "objectId"))]
    object_id: String,
    multihash: String,
}

#[derive(serde::Deserialize, Debug)]
struct Org {
    id: OrgId,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("'git' command not found")]
    GitNotFound,

    #[error("client request failed: {0}")]
    Handle(#[from] client::handle::Error),

    #[error(transparent)]
    Channel(#[from] mpsc::error::SendError<Urn>),

    #[error(transparent)]
    FromHex(#[from] rustc_hex::FromHexError),
}

/// Run the Node.
pub fn run(rt: tokio::runtime::Runtime, options: Options) -> anyhow::Result<()> {
    let git_version = std::process::Command::new("git")
        .arg("version")
        .output()
        .map_err(|_| Error::GitNotFound)?
        .stdout;
    tracing::info!(target: "org-node", "{}", std::str::from_utf8(&git_version).unwrap().trim());

    let profile = Profile::from_root(&options.rad_home, None)
        .context("failed to initialize Radicle profile")?;
    let paths = profile.paths().clone();

    let key = load_or_create_secret_key(&paths)?;
    let peer_id = librad::PeerId::from(&key);
    let signer = client::Signer::new(key);
    let client = Client::new(
        paths,
        signer,
        client::Config {
            listen: options.listen,
            bootstrap: options.bootstrap.clone(),
            ..client::Config::default()
        },
    );
    let handle = client.handle();

    tracing::info!("Peer ID = {}", peer_id);
    tracing::info!(bootstrap = ?options.bootstrap, "bootstrap");

    // Queue of projects to track.
    let (urn_sender, urn_receiver) = mpsc::channel(256);

    for project in options.projects {
        urn_sender.try_send(project).unwrap();
    }

    let client_task = rt.spawn(client.run(rt.handle().clone()));
    let track_task = rt.spawn(track_projects(handle, urn_receiver));

    tracing::info!(target: "org-node", "Listening on {}...", options.listen);

    let result = rt.block_on(async {
        tokio::select! {
            result = client_task => result,
            result = track_task => result,
        }
    });

    if let Err(err) = result {
        tracing::info!(target: "org-node", "Task failed: {}", err);
    }
    tracing::info!(target: "org-node", "Exiting..");

    Ok(())
}

/// Track projects sent via the queue.
///
/// This function only returns if the channels it uses to communicate with other
/// tasks are closed.
async fn track_projects(mut handle: client::Handle, mut urn_receiver: mpsc::Receiver<Urn>) {
    // URNs to track are added to the back of this queue, and taken from the front.
    let mut work = VecDeque::new();

    loop {
        // Drain ascynchronous tracking queue, moving URNs to work queue.
        // This ensures that we aren't only retrying existing URNs that have timed out
        // and have been added back to the work queue.
        loop {
            match urn_receiver.try_recv() {
                Ok(urn) => {
                    work.push_back(urn.clone());
                    tracing::debug!(target: "org-node", "{}: Added to the work queue ({})", urn, work.len());
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    tracing::debug!(target: "org-node", "Channel is empty");
                    break;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    tracing::info!(target: "org-node", "Queue shutdown, exiting task");
                    return;
                }
            }
        }

        // If we have something to work on now, work on it, otherwise block on the
        // async tracking queue. We do this to avoid spin-looping, since the queue
        // is drained without blocking.
        let urn = if let Some(front) = work.pop_front() {
            front
        } else if let Some(urn) = urn_receiver.recv().await {
            urn
        } else {
            // This only happens if the tracking queue was closed from another task.
            // In this case we expect the condition to be caught in the next iteration.
            continue;
        };
        tracing::info!(target: "org-node", "{}: Attempting to track.. (work={})", urn, work.len());

        // If we fail to track, re-add the URN to the back of the queue.
        match handle.track_project(urn.clone()).await {
            Ok(reply) => match reply {
                Ok(Some(peer_id)) => {
                    tracing::info!(target: "org-node", "{}: Fetched from {}", urn, peer_id);
                }
                Ok(None) => {
                    tracing::debug!(target: "org-node", "{}: Nothing to do", urn);
                }
                Err(client::TrackProjectError::NotFound) => {
                    tracing::info!(target: "org-node", "{}: Not found", urn);
                    work.push_back(urn);
                }
            },
            Err(client::handle::Error::Timeout(err)) => {
                tracing::info!(target: "org-node", "{}: Tracking timed out: {}", urn, err);
                work.push_back(urn);
            }
            Err(err) => {
                tracing::error!(target: "org-node", "Tracking handle failed, exiting task ({})", err);
                return;
            }
        }
    }
}

fn load_or_create_secret_key(
    rad_paths: &librad::paths::Paths,
) -> anyhow::Result<librad::SecretKey> {
    use librad::keystore::SecretKeyExt as _;
    use std::io::Write as _;
    use std::os::unix::prelude::PermissionsExt as _;

    let keys_dir = rad_paths.keys_dir();
    std::fs::create_dir_all(keys_dir)?;
    let key_path = keys_dir.join("identity.key");

    if key_path.exists() {
        let contents = std::fs::read(key_path)?;
        let secret_key = (librad::SecretKey::from_bytes_and_meta(contents.into(), &()))?;
        Ok(secret_key)
    } else {
        let mut file = File::create(key_path)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        let secret_key = librad::SecretKey::new();
        file.write_all(secret_key.as_ref())?;
        Ok(secret_key)
    }
}
