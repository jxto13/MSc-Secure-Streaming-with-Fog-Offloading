// use std::{error::Error, os::fd::RawFd};

// use log::info;

// pub fn start_router(tun_fd: RawFd) -> Result<(), Box<dyn Error>> {
//     // Initialize the router here
//     log::info!("Starting router...");

//     use futures::prelude::*;
//     use libp2p::{noise, ping, swarm::SwarmEvent, tcp, yamux, Multiaddr};
//     use std::time::Duration;
//     use tokio::runtime::Runtime;

//     let rt = Runtime::new()?; // Create the Tokio runtime

//     rt.block_on(async {
//         let mut swarm = libp2p::SwarmBuilder::with_new_identity()
//             .with_tokio()
//             .with_tcp(
//                 tcp::Config::default(),
//                 noise::Config::new,
//                 yamux::Config::default,
//             )?
//             .with_behaviour(|_| ping::Behaviour::default())?
//             .with_swarm_config(|cfg| {
//                 cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
//             })
//             .build();

//         swarm.listen_on("/ip6/::/tcp/0".parse()?)?;

//         if let Some(addr) = std::env::args().nth(1) {
//             let remote: Multiaddr = addr.parse()?;
//             swarm.dial(remote)?;
//             info!("Dialed {addr}")
//         }

//         loop {
//             match swarm.select_next_some().await {
//                 SwarmEvent::NewListenAddr { address, .. } => {
//                     info!("Listening on {address:?}")
//                 }
//                 SwarmEvent::Behaviour(event) => info!("{event:?}"),
//                 uncaught => {
//                     info!("Uncaught event: {uncaught:?}");
//                 }
//             }
//         }
//     })
// }

use pnet_datalink;

use log::{error, info};
use std::{
    error::Error,
    net::{Ipv4Addr, Ipv6Addr},
    os::fd::RawFd,
};

use futures::stream::StreamExt;
use libp2p::{
    kad::{self, store::MemoryStore, Mode},
    mdns,
    multiaddr::Protocol,
    noise,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr,
};
use tokio::{
    io::{self, AsyncBufReadExt},
    select,
};
pub fn start_router(tun_fd: RawFd) -> Result<(), Box<dyn Error>> {
    // Initialize the router here
    log::info!("Starting router...");

    use futures::prelude::*;
    use libp2p::{noise, ping, swarm::SwarmEvent, tcp, yamux, Multiaddr};
    use std::time::Duration;
    use tokio::runtime::Runtime;

    let rt = Runtime::new()?; // Create the Tokio runtime
                              // We create a custom network behaviour that combines Kademlia and mDNS.
    #[derive(NetworkBehaviour)]
    struct Behaviour {
        kademlia: kad::Behaviour<MemoryStore>,
        mdns: mdns::tokio::Behaviour,
    }

    rt.block_on(async {

        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_quic()
            .with_behaviour(|key| {
                Ok(Behaviour {
                    kademlia: kad::Behaviour::new(
                        key.public().to_peer_id(),
                        MemoryStore::new(key.public().to_peer_id()),
                    ),
                    mdns: mdns::tokio::Behaviour::new(
                        mdns::Config::default(),
                        key.public().to_peer_id(),
                    )?,
                })
            })?
            .build();

        swarm.behaviour_mut().kademlia.set_mode(Some(Mode::Server));

        // Read full lines from stdin
        let mut stdin = io::BufReader::new(io::stdin()).lines();

        // Listen on all interfaces and whatever port the OS assigns.
        swarm.listen_on("/ip6/::/udp/0/quic-v1".parse()?)?;

        // Kick it off.
        loop {
            select! {
                Ok(Some(line)) = stdin.next_line() => {
                    let mut args = line.split(' ');
                    match args.next() {
                        Some("LISTEN") => {
                            match args.next() {
                                Some(addr) => {
                                    match addr.parse::<Multiaddr>() {
                                        Ok(ma) => {
                                            match swarm.listen_on(ma.clone()) {
                                                Ok(_) => info!("Listening on: {:?}", ma),
                                                Err(e) => error!("Failed to listen on {:?}: {e}", ma),
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to parse multiaddr: {e}");
                                        }
                                    }
                                }
                                None => {
                                    error!("Expected multiaddr");
                                }
                            }
                        }
                        _ => {
                            handle_input_line(&mut swarm.behaviour_mut().kademlia, line);
                        }
                    }
                }
                event = swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("Listening in {address:?}");
                    },
                    SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, multiaddr) in list {
                            info!("Discovered peer {peer_id:?} at {multiaddr:?}");
                            swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr);
                        }
                    }
                    SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { result, ..})) => {
                        match result {
                            kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { key, providers, .. })) => {
                                for peer in providers {
                                    info!(
                                        "Peer {peer:?} provides key {:?}",
                                        std::str::from_utf8(key.as_ref()).unwrap()
                                    );
                                }
                            }
                            kad::QueryResult::GetProviders(Err(err)) => {
                                error!("Failed to get providers: {err:?}");
                            }
                            kad::QueryResult::GetRecord(Ok(
                                kad::GetRecordOk::FoundRecord(kad::PeerRecord {
                                    record: kad::Record { key, value, .. },
                                    ..
                                })
                            )) => {
                                info!(
                                    "Got record {:?} {:?}",
                                    std::str::from_utf8(key.as_ref()).unwrap(),
                                    std::str::from_utf8(&value).unwrap(),
                                );
                            }
                            kad::QueryResult::GetRecord(Ok(_)) => {}
                            kad::QueryResult::GetRecord(Err(err)) => {
                                error!("Failed to get record: {err:?}");
                            }
                            kad::QueryResult::PutRecord(Ok(kad::PutRecordOk { key })) => {
                                info!(
                                    "Successfully put record {:?}",
                                    std::str::from_utf8(key.as_ref()).unwrap()
                                );
                            }
                            kad::QueryResult::PutRecord(Err(err)) => {
                                error!("Failed to put record: {err:?}");
                            }
                            kad::QueryResult::StartProviding(Ok(kad::AddProviderOk { key })) => {
                                info!(
                                    "Successfully put provider record {:?}",
                                    std::str::from_utf8(key.as_ref()).unwrap()
                                );
                            }
                            kad::QueryResult::StartProviding(Err(err)) => {
                                error!("Failed to put provider record: {err:?}");
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}

fn handle_input_line(kademlia: &mut kad::Behaviour<MemoryStore>, line: String) {
    let mut args = line.split(' ');

    match args.next() {
        Some("GET") => {
            let key = {
                match args.next() {
                    Some(key) => kad::RecordKey::new(&key),
                    None => {
                        error!("Expected key");
                        return;
                    }
                }
            };
            kademlia.get_record(key);
        }
        Some("GET_PROVIDERS") => {
            let key = {
                match args.next() {
                    Some(key) => kad::RecordKey::new(&key),
                    None => {
                        error!("Expected key");
                        return;
                    }
                }
            };
            kademlia.get_providers(key);
        }
        Some("PUT") => {
            let key = {
                match args.next() {
                    Some(key) => kad::RecordKey::new(&key),
                    None => {
                        error!("Expected key");
                        return;
                    }
                }
            };
            let value = {
                match args.next() {
                    Some(value) => value.as_bytes().to_vec(),
                    None => {
                        error!("Expected value");
                        return;
                    }
                }
            };
            let record = kad::Record {
                key,
                value,
                publisher: None,
                expires: None,
            };
            kademlia
                .put_record(record, kad::Quorum::One)
                .expect("Failed to store record locally.");
        }
        Some("PUT_PROVIDER") => {
            let key = {
                match args.next() {
                    Some(key) => kad::RecordKey::new(&key),
                    None => {
                        error!("Expected key");
                        return;
                    }
                }
            };

            kademlia
                .start_providing(key)
                .expect("Failed to start providing key");
        }
        _ => {
            error!("expected GET, GET_PROVIDERS, PUT or PUT_PROVIDER");
        }
    }
}
