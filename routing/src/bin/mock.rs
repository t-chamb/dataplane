use dplane_rpc::log::{Level, LogConfig, init_dplane_rpc_log};
use routing::cpi::{CpiConf, start_cpi};
use routing::fib::fibtable::FibTableWriter;
use routing::fib::fibtype::FibId;
use routing::interfaces::iftablerw::{IfTableReader, IfTableWriter};
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use tracing::{debug, warn};

use net::eth::mac::Mac;
use routing::atable::atablerw::AtableWriter;
use routing::interfaces::interface::IfDataEthernet;
use routing::interfaces::interface::IfState;
use routing::interfaces::interface::IfType;
use routing::interfaces::interface::Interface;

use afpacket::sync::RawPacketStream;
use nom::HexDisplay;
use std::io::Read;
//use std::io::Write;

fn populate_interfaces(iftw: &mut IfTableWriter) {
    /* create Lo */
    let mut lo = Interface::new("Loopback", 1);
    lo.set_admin_state(IfState::Up);
    lo.set_oper_state(IfState::Up);
    lo.set_description("Main loopback interface");
    lo.set_iftype(IfType::Loopback);
    iftw.add_interface(lo);

    /* create Eth0 */
    let mut eth0 = Interface::new("eth0", 20);
    eth0.set_admin_state(IfState::Up);
    eth0.set_oper_state(IfState::Up);
    eth0.set_description("Link to spine");
    eth0.set_iftype(IfType::Ethernet(IfDataEthernet {
        mac: Mac::from([0x12, 0x06, 0x6f, 0x60, 0x2c, 0x84]),
    }));
    iftw.add_interface(eth0);
}

fn process_packets() {
    let mut ps = RawPacketStream::new().unwrap();
    ps.set_non_blocking();
    ps.bind("eth0");
    let mut buf = [0u8; 2048];

    loop {
        if let Ok(bytes) = ps.read(&mut buf) {
            println!("Packet ({bytes} octets):");
            println!("{}", buf[0..bytes].to_hex(24));
            // write it back
            //let w = ps.write(&buf[0..bytes]);
            //println!("wrote {:?}", w);
        } else {
            /// Use a poller instead
            let ten_millis = Duration::from_millis(100);
            thread::sleep(ten_millis);
        }
    }
}

fn main() {
    let conf = CpiConf {
        rpc_loglevel: Some("debug".to_string()),
        cpi_sock_path: Some("/var/run/frr/hh/dataplane.sock".to_string()),
        cli_sock_path: Some("/tmp/dataplane_ctl.sock".to_string()),
    };

    let loglevel = conf
        .rpc_loglevel
        .as_ref()
        .map(|level| Level::from_str(level).expect("Wrong log level"))
        .unwrap_or_else(|| Level::DEBUG);

    /* set loglevel for RPC */
    let mut cfg = LogConfig::new(loglevel);
    cfg.display_thread_names = true;
    cfg.show_line_numbers = true;
    cfg.display_target = true;
    init_dplane_rpc_log(&cfg);

    /* create iftable */
    let (mut iftw, iftr) = IfTableWriter::new();
    //    populate_interfaces(&mut iftw);
    //    if let Some(iftable) = iftr.enter() {
    //        debug!("\n{}", *iftable);
    //    }

    /* create FIB table and writer and reader */
    let (mut fibtw, fibtr) = FibTableWriter::new();

    /* create atable */
    let (mut atablew, atabler) = AtableWriter::new();

    /*
        /* create routing database */
        let db = RoutingDb::new();

        /* Todo: this needs to be configured */
        if let Ok(mut vtep) = db.vtep.write() {
            vtep.set_ip(IpAddr::from_str("7.0.0.1").unwrap());
            vtep.set_mac(Mac::from([0x02, 0, 0, 0, 0, 0xab]));
        }
    */

    /* start CPI */
    let cpi = start_cpi(&conf, fibtw, iftw, atabler).expect("Failed to start CPI");

    //  process_packets();

    loop {
        let nap = Duration::from_secs(2);
        thread::sleep(nap);

        /*
        if let Some(fibtable) = fibt_r.enter() {
            debug!("Num fibs: {}", fibtable.deref().len());
            if let Some(fibr) = fibtable.get_fib(FibId::from_vrfid(0)) {
                if let Some(fib) = fibr.enter() {
                    debug!(
                        "Fib({}) v4:{} v6:{}",
                        fib.version(),
                        fib.len_v4(),
                        fib.len_v6()
                    );
                    for (prefix, fibgroup) in fib.iter_v4() {
                        debug!("{prefix:?} {fibgroup}");
                    }
                } else {
                    warn!("Could not enter fib with id {}", 0);
                }
            } else {
                warn!("No fib with id {}", 0);
            }
        } */
    }

    if let Some(handle) = cpi.handle {
        let _ = handle.join();
    }
}
