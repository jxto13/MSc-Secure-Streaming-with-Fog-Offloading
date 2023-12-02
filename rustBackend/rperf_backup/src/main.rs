/*
 * Copyright (C) 2021 Evtech Solutions, Ltd., dba 3D-P
 * Copyright (C) 2021 Neil Tallim <neiltallim@3d-p.com>
 *
 * This file is part of rperf.
 *
 * rperf is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * rperf is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with rperf.  If not, see <https://www.gnu.org/licenses/>.
 */

mod client;
mod protocol;
mod server;
mod stream;
mod utils;

use std::error::Error;

use log::{debug, error};

pub fn start_server() -> Result<(), Box<dyn Error>> {

    debug!("beginning normal operation...");
    let service = server::serve();
    if let Err(err) = &service {
        error!("unable to run server: {}", err);
    }
    service
}

pub fn start_client(ipv6: &str) -> Result<(), Box<dyn Error>> {
    debug!("connecting to server {} ...", ipv6);

    let execution = client::execute(ipv6);
    if let Err(err) = &execution {
        error!("unable to run client: {}", err);
    }

    execution
}


