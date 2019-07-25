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

mod reg;

use std::net::Ipv4Addr;
use std::path::PathBuf;
use structopt::StructOpt;
use systemd::{daemon, journal};

#[derive(StructOpt)]
pub struct Options {
    #[structopt(short="v", long="verbose", help="Verbose logging?")]
    verbose: bool,
    #[structopt(short="b", long="broadcast", help="Broadcast address to use")]
    broadcast: Option<Ipv4Addr>,
    #[structopt(short="i", long="interface", default_value="eth0", help="Network interface to use",
                parse(try_from_str = "mlzutil::net::iface::parse_interface"))]
    interface: interfaces::Interface,
    #[structopt(short="a", long="additional-cache", help="Additional cache")]
    addcache: Option<String>,
    #[structopt(short="t", long="network-timeout", default_value="2.0", help="Network timeout (sec)")]
    timeout: f64,
    #[structopt(short="l", long="ttl", default_value="60.0",
                help="Time to live for cache registration (sec)")]
    ttl: f64,
    #[structopt(short="F", long="check-file", help="If given, exit when this file doesn't exist")]
    checkfile: Option<PathBuf>,
    #[structopt(short="I", long="identifier", help="Explicit PNP identifier to register")]
    identifier: Option<String>,
    #[structopt(short="S", long="setupname", help="Explicit setup name to register")]
    setupname: Option<String>,
}

fn main() {
    let opts = Options::from_args();

    journal::JournalLog::init().expect("failed to open journal for logging");
    log::set_max_level(if opts.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    });

    match reg::Registrar::new(opts) {
        Err(err) => {
            log::error!("during startup: {}", err);
            std::process::exit(1);
        }
        Ok(reg) => {
            let _ = daemon::notify(false, Some((daemon::STATE_READY, "1")).iter());
            if let Err(err) = reg.run() {
                log::error!("in handler: {}", err);
                std::process::exit(1);
            }
        }
    }
}
