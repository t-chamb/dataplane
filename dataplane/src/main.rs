// SPDX-License-Identifier: Apache-2.0
// Copyright Open Network Fabric Authors

#![deny(clippy::all, clippy::pedantic)]
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_crate_level_docs)]

use crate::args::CmdArgs;
use clap::Parser;
use tracing::{error, info};

mod args;
mod drivers;
mod nat;
mod packet_processor;

use drivers::dpdk::DriverDpdk;
use drivers::kernel::DriverKernel;
use net::buffer::{PacketBufferMut, TestBuffer};
use net::packet::Packet;
use pipeline::DynPipeline;
use pipeline::sample_nfs::PacketDumper;

use packet_processor::{setup_routing_pipeline, start_router};
use routing::router::Router;

fn init_logging() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_thread_names(true)
        .init();
}

fn setup_pipeline<Buf: PacketBufferMut>() -> DynPipeline<Buf> {
    let pipeline = DynPipeline::new();
    if false {
        /* replace false by true to try filters and write your own */
        let custom_filter = |_packet: &Packet<Buf>| -> bool {
            /* your own filter here */
            true
        };
        pipeline.add_stage(PacketDumper::new(
            "default",
            true,
            Some(Box::new(custom_filter)),
        ))
    } else {
        pipeline.add_stage(PacketDumper::new("default", true, None))
    }
}

fn setup_router<Buf: PacketBufferMut>() -> DynPipeline<Buf> {
    let (router, pipeline) = start_router("demo").expect("Failed to start router");
    pipeline
}

/*
fn generate_pipeline_builder<Buf: PacketBufferMut>(
    router: &Router) -> impl Fn() -> DynPipeline<Buf> {
    let pipeline = setup_routing_pipeline(
        router.get_iftabler(),
        router.get_fibtr(),
        router.get_atabler().expect("Failed to get atable reader"),
    );
    make_builder(pipeline)
}

fn make_builder<Buf: PacketBufferMut>(pipeline: DynPipeline<Buf>) -> impl Fn() -> DynPipeline<Buf> {
    move || pipeline
}
 */

fn main() {
    init_logging();
    info!("Starting gateway process...");

    let (stop_tx, stop_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || stop_tx.send(()).expect("Error sending SIGINT signal"))
        .expect("failed to set SIGINT handler");

    /* parse cmd line args */
    let args = CmdArgs::parse();

    let router = Router::new("demo");
    let pipeline = setup_routing_pipeline(
        router.get_iftabler(),
        router.get_fibtr(),
        router.get_atabler(),
    );
    let builder = move || pipeline;
    //let builder = move || setup_pipeline::<TestBuffer>();

    //let (router, pipeline) = start_router::<Buf>("demo-router").expect("Failed to start router");

    /* start driver */
    match args.get_driver_name() {
        "dpdk" => {
            info!("Using driver DPDK...");
            DriverDpdk::start(args.eal_params(), &setup_pipeline);
        }
        "kernel" => {
            info!("Using driver kernel...");
            DriverKernel::start(args.kernel_params(), builder);
        }
        other => {
            error!("Unknown driver '{other}'. Aborting...");
            std::process::exit(0);
        }
    }

    stop_rx.recv().expect("failed to receive stop signal");
    info!("Shutting down dataplane");
    std::process::exit(0);
}
