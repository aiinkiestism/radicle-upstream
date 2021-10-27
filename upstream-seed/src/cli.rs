/// Upstream sed node
#[derive(Debug, structopt::StructOpt)]
pub struct Args {
    /// listen on the following address for peer messages
    #[structopt(long, default_value = "0.0.0.0:8776")]
    pub listen: std::net::SocketAddr,

    /// path to store radicle profile data
    #[structopt(long)]
    pub rad_home: std::path::PathBuf,

    /// list of bootstrap peers, eg.
    /// 'f00...@seed1.example.com:12345,bad...@seed2.example.com:12345'
    #[structopt(long, parse(try_from_str = parse_bootstrap))]
    pub bootstrap: Option<Vec<(librad::PeerId, std::net::SocketAddr)>>,

    /// project URN to track. can be specified multiple times
    #[structopt(long)]
    pub project: Vec<link_identities::git::Urn>,
}

fn parse_bootstrap(value: &str) -> Result<(librad::PeerId, std::net::SocketAddr), String> {
    use std::net::ToSocketAddrs as _;
    use std::str::FromStr as _;

    let parts = value.splitn(2, '@').collect::<Vec<_>>();
    let id = librad::PeerId::from_str(parts[0]).map_err(|e| e.to_string())?;
    let addr = parts[1]
        .to_socket_addrs()
        .map(|mut a| a.next())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Could not resolve peer address".to_owned())?;

    Ok((id, addr))
}
