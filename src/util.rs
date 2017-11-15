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

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::fs::{self, DirBuilder};
use std::path::{Path, PathBuf};
use dns_lookup;
use hostname;
use interfaces;

/// Shortcut for canonicalizing a path, if possible.
pub fn abspath(path: impl AsRef<Path>) -> PathBuf {
    path.as_ref().canonicalize().unwrap_or_else(|_| path.as_ref().into())
}

/// mkdir -p utility.
pub fn ensure_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if path.as_ref().is_dir() {
        return Ok(());
    }
    DirBuilder::new().recursive(true).create(path)
}

/// Write a PID file.
pub fn write_pidfile(pid_path: impl AsRef<Path>) -> io::Result<()> {
    ensure_dir(&pid_path)?;
    let file = pid_path.as_ref().join("cache_rs.pid");
    let my_pid = fs::read_link("/proc/self")?;
    let my_pid = my_pid.to_str().unwrap();
    fs::write(file, my_pid.as_bytes())
}

/// Remove a PID file.
pub fn remove_pidfile(pid_path: impl AsRef<Path>) {
    let file = Path::new(pid_path.as_ref()).join("cache_rs.pid");
    let _ = fs::remove_file(file);
}

/// Get best-effort fully-qualified hostname.
pub fn getfqdn() -> String {
    let hostname = hostname::get_hostname().unwrap_or_else(|| "localhost".into());
    let mut candidates = Vec::new();
    for addr in dns_lookup::lookup_host(&hostname).unwrap_or_default() {
        if let Ok(name) = dns_lookup::lookup_addr(&addr) {
            if name.contains('.') {
                return name;
            }
            candidates.push(name);
        }
    }
    candidates.pop().unwrap_or_else(|| "localhost".into())
}

/// Get a valid interface name.
pub fn parse_interface(ifname: &str) -> Result<interfaces::Interface, String> {
    match interfaces::Interface::get_by_name(ifname) {
        Ok(Some(iface)) => Ok(iface),
        Ok(None) => Err("no such interface".into()),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Extract the Ipv4Addr from the given IpAddr.
pub fn unwrap_ipv4(addr: IpAddr) -> Ipv4Addr {
    match addr {
        IpAddr::V6(_) => panic!("IPv4 address required"),
        IpAddr::V4(ip) => ip
    }
}

/// Find the IPv4 address and netmask in the given list of addresses.
pub fn ipv4_addr(addresses: &[interfaces::Address]) -> Option<(Ipv4Addr, Ipv4Addr)> {
    addresses.iter().find(|ad| ad.kind == interfaces::Kind::Ipv4)
        .map(|ad| (unwrap_ipv4(ad.addr.unwrap().ip()),
                   unwrap_ipv4(ad.mask.unwrap().ip())))
}

/// Determine IPv4 address of a host name.
pub fn lookup_ipv4(host: &str) -> Option<Ipv4Addr> {
    for addr in (host, 0).to_socket_addrs().ok()? {
        if let SocketAddr::V4(v4addr) = addr {
            return Some(*v4addr.ip());
        }
    }
    None
}
