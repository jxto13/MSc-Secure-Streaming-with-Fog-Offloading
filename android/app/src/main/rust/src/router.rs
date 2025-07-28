use std::{error::Error, os::fd::RawFd};

use log::info;

pub fn start_router(tun_fd: RawFd) -> Result<(), Box<dyn Error>> {
    // Initialize the router here
    log::info!("Starting router...");

    use futures::prelude::*;
    use libp2p::{noise, ping, swarm::SwarmEvent, tcp, yamux, Multiaddr};
    use std::time::Duration;
    use tokio::runtime::Runtime;

    let rt = Runtime::new()?; // Create the Tokio runtime

    rt.block_on(async {
        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|_| ping::Behaviour::default())?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build();

        swarm.listen_on("/ip6/::/tcp/0".parse()?)?;

        if let Some(addr) = std::env::args().nth(1) {
            let remote: Multiaddr = addr.parse()?;
            swarm.dial(remote)?;
            info!("Dialed {addr}")
        }

        loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {address:?}")
                }
                SwarmEvent::Behaviour(event) => info!("{event:?}"),
                uncaught => {
                    info!("Uncaught event: {uncaught:?}");
                }
            }
        }
    })
}
