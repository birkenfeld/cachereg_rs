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
//   Georg Brandl <g.brandl@fz-juelich.de>
//
// *****************************************************************************

use std::error::Error;
use std::net::{Ipv4Addr, UdpSocket};
use std::thread;
use std::time::Duration;
use chan_signal;

use util;

const CACHE_PORT: u16 = 14869;

use Options;

pub struct Registrar {
    opts: Options,
    msg: String,
    sock: UdpSocket,
    addrs: Vec<Ipv4Addr>,
}

impl Registrar {
    pub fn new(opts: Options) -> Result<Self, Box<Error>> {
        let (query_msg, msg) = Self::registration_msgs(&opts);
        let (ip, mask) = util::ipv4_addr(&opts.interface.addresses)
            .ok_or("no IP address found for this interface")?;
        let sock = UdpSocket::bind((ip, 0))?;
        sock.set_broadcast(true)?;
        sock.set_read_timeout(Some(Duration::from_millis(opts.timeout as u64 * 1000)))?;
        let mut addrs = Vec::new();

        let broadcast_addr = Self::broadcast_addr(&opts, (ip, mask));
        if let Some(addr) = Self::find_unicast_addr(&sock, broadcast_addr, query_msg) {
            addrs.push(addr);
        } else {
            addrs.push(broadcast_addr);
        }

        if let Some(addr) = opts.addcache.as_ref().and_then(|a| util::lookup_ipv4(&a)) {
            addrs.push(addr);
        }

        Ok(Registrar { opts, msg, sock, addrs })
    }

    pub fn run(self) {
        info!("starting registration loop...");
        loop {
            if !self.opts.checkfile.as_ref().map_or(true, |f| f.exists()) {
                info!("file {} not present anymore, exiting",
                      self.opts.checkfile.as_ref().unwrap().display());
                chan_signal::kill_this(chan_signal::Signal::TERM);
            }

            debug!("sending registration message");
            for &addr in &self.addrs {
                if let Err(e) = self.sock.send_to(self.msg.as_bytes(), (addr, CACHE_PORT)) {
                    warn!("error sending message to {}: {}", addr, e);
                }
            }

            let sleep_ms = 2000.max(self.opts.ttl as u64 * 1000) - 2000;
            thread::sleep(Duration::from_millis(sleep_ms));
        }
    }

    fn registration_msgs(opts: &Options) -> (String, String) {
        let fqdn = util::getfqdn();
        let identifier = opts.identifier.as_ref().unwrap_or(&fqdn);
        let setupname = opts.setupname.as_ref().map_or(fqdn.split('.').next().unwrap(), |s| &*s);
        let prefix = format!("+{}@se/{}/nicos", opts.ttl, identifier);
        (format!("{}/setupname='{}'\n{}/setupname?\n", prefix, setupname, prefix),
         format!("{}/setupname='{}'\n", prefix, setupname))
    }

    fn broadcast_addr(opts: &Options, (ip, mask): (Ipv4Addr, Ipv4Addr)) -> Ipv4Addr {
        opts.broadcast.unwrap_or_else(|| Ipv4Addr::from(u32::from(ip) | !u32::from(mask)))
    }

    fn find_unicast_addr(sock: &UdpSocket, broadcast_addr: Ipv4Addr, query: String)
                         -> Option<Ipv4Addr> {
        let mut buf = [0; 2048];
        if let Err(e) = sock.send_to(query.as_bytes(), (broadcast_addr, CACHE_PORT)) {
            warn!("error sending query to {}: {}", broadcast_addr, e);
            return None;
        }
        if let Ok((_, src_addr)) = sock.recv_from(&mut buf) {
            debug!("got unicast reply from {:?}", src_addr);
            Some(util::unwrap_ipv4(src_addr.ip()))
        } else {
            debug!("no unicast reply, continuing to broadcast");
            None
        }
    }
}
