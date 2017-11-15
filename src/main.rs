// *****************************************************************************
//
// This program is free software; you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation; either version 2 of the License, or (at your option) any later
// version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
// details.
//
// You should have received a copy of the GNU General Public License along with
// this program; if not, write to the Free Software Foundation, Inc.,
// 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
//
// Module authors:
//   Alexander Lenz <alexander.lenz@frm2.tum.de>
//   Georg Brandl <g.brandl@fz-juelich.de>
//
// *****************************************************************************

// Use the system allocator instead of jemalloc.
// This allows us to build with the i586 target on Debian 7.
#![feature(alloc_system, global_allocator, allocator_api)]
extern crate alloc_system;
use alloc_system::System;
#[global_allocator]
static A: System = System;

#[macro_use]
extern crate log;
extern crate mlzlog;
extern crate byteorder;
#[macro_use]
extern crate structopt;
extern crate interfaces;
extern crate itertools;
extern crate daemonize;
extern crate chan_signal;
extern crate hostname;
extern crate dns_lookup;

mod reg;
mod util;

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::{thread, process};
use structopt::StructOpt;

/// A forwarder for Beckhoff ADS and UDP connections.
#[derive(StructOpt)]
pub struct Options {
    #[structopt(short="v", long="verbose", help="Verbose logging?")]
    verbose: bool,
    #[structopt(short="d", long="daemonize", help="Run as daemon?")]
    daemonize: bool,
    #[structopt(short="b", long="broadcast", help="Broadcast address to use")]
    broadcast: Option<Ipv4Addr>,
    #[structopt(short="i", long="interface", default_value="eth0", help="Network interface to use",
                parse(try_from_str = "util::parse_interface"))]
    interface: interfaces::Interface,
    #[structopt(short="a", long="additional-cache", help="Additional cache")]
    addcache: Option<String>,
    #[structopt(short="t", long="network-timeout", default_value="2.0", help="Network timeout (sec)")]
    timeout: f64,
    #[structopt(short="l", long="ttl", default_value="60.0",
                help="Time to live for cache registration (sec)")]
    ttl: f64,
    #[structopt(short="u", long="user", help="User name for daemon")]
    user: Option<String>,
    #[structopt(short="g", long="group", help="Group name for daemon")]
    group: Option<String>,
    #[structopt(short="p", long="pid-file", default_value="/var/run/cachereg.pid",
                help="PID file for daemon")]
    pidfile: String,
    #[structopt(short="L", long="log-dir", default_value="/var/log", help="Logfile directory")]
    logdir: String,
    #[structopt(short="F", long="check-file", help="If given, exit when this file doesn't exist")]
    checkfile: Option<PathBuf>,
    #[structopt(short="I", long="identifier", help="Explicit PNP identifier to register")]
    identifier: Option<String>,
    #[structopt(short="S", long="setupname", help="Explicit setup name to register")]
    setupname: Option<String>,
}

fn main() {
    let opts = Options::from_args();

    let logdir = util::abspath(&opts.logdir);
    let pidpath = util::abspath(&opts.pidfile);
    if opts.daemonize {
        let mut daemon = daemonize::Daemonize::new();
        if let Some(user) = &opts.user {
            daemon = daemon.user(&**user);
        }
        if let Some(group) = &opts.group {
            daemon = daemon.group(&**group);
        }
        if let Err(err) = daemon.start() {
            eprintln!("could not daemonize process: {}", err);
        }
    }
    if let Err(err) = mlzlog::init(Some(&logdir), "cachereg", false, opts.verbose, true) {
        eprintln!("could not initialize logging: {}", err);
    }
    if let Err(err) = util::write_pidfile(&pidpath) {
        error!("could not write PID file: {}", err);
    }
    // handle SIGINT and SIGTERM
    let signal_chan = chan_signal::notify(&[chan_signal::Signal::INT,
                                            chan_signal::Signal::TERM]);

    match reg::Registrar::new(opts) {
        Err(err) => {
            error!("during startup: {}", err);
            process::exit(1);
        }
        Ok(reg) => {
            thread::spawn(|| reg.run());
        }
    }

    // wait for a signal to finish
    signal_chan.recv().unwrap();
    info!("quitting...");
    util::remove_pidfile(pidpath);
}
