// Copyright © 2021 The Radicle Upstream Contributors
//
// This file is part of radicle-upstream, distributed under the GPLv3
// with Radicle Linking Exception. For full terms see the included
// LICENSE file.

//! Configuration vital to the setup and alteration of the application.

use std::{env, io, path};

use directories::ProjectDirs;

use librad::profile::ProfileId;

/// Errors when setting up configuration paths and variables.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Exception during I/O actions.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Couldn't join file paths to calculate a new PATH variable.
    #[error(transparent)]
    Path(#[from] env::JoinPathsError),
    /// Couldn't get an environment variable's value.
    #[error(transparent)]
    Var(#[from] env::VarError),
    /// We couldn't get the executable path.
    #[error("we were not able to find the executable path's parent directory")]
    MissingExePath(path::PathBuf),
}

/// Returns the directories to locate all application state.
#[must_use]
pub fn dirs() -> ProjectDirs {
    ProjectDirs::from("xyz", "radicle", "radicle-upstream").expect("couldn't build dirs")
}

/// Returns the directory for the application store
pub fn store_dir(profile_id: &ProfileId, rad_home: Option<&path::Path>) -> path::PathBuf {
    let store_root = match rad_home {
        None => {
            let dirs = dirs();
            dirs.data_dir().to_path_buf()
        },
        Some(root) => root.to_path_buf(),
    };
    store_root.join(profile_id.as_str()).join("store")
}

/// Returns the path to a folder containing helper binaries.
///
/// # Errors
///
///   * Could not get the user home path from the HOME env variable
pub fn bin_dir() -> Result<path::PathBuf, Error> {
    let home_dir;
    #[cfg(windows)]
    {
        home_dir = std::env::var("USERPROFILE")?;
    }
    #[cfg(unix)]
    {
        home_dir = std::env::var("HOME")?;
    }

    Ok(path::Path::new(&home_dir).join(".radicle/bin"))
}

/// Returns path to the directory containing the proxy binary.
///
/// # Errors
///
///   * Could not determine the path of this binary.
pub fn proxy_path() -> Result<path::PathBuf, Error> {
    let exe_path = std::env::current_exe()?;

    Ok(exe_path
        .parent()
        .ok_or_else(|| Error::MissingExePath(exe_path.clone()))?
        .to_owned())
}
