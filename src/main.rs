#![warn(clippy::nursery, clippy::pedantic)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions
)]

use std::{fs, path::PathBuf};

use clap::Parser;

pub mod class;
pub mod class_loader;
pub mod virtual_machine;

#[derive(Parser)]
struct Args {
    filename: PathBuf,
    #[clap(short, long)]
    run: bool,
    #[clap(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    let bytes = fs::read(args.filename).unwrap();
    // let bytes = [
    //     0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 0, 0, 3, 1, 0, 2, 0x30, 0x30, 3, 0, 0, 0, 0xFF, 0, 0, 0,
    //     1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // ];
    let class = class_loader::load_class(&mut bytes.into_iter(), args.verbose).unwrap();
    if args.verbose {
        println!("{class:#?}");
    }
    if args.run {
        virtual_machine::start_vm(class, args.verbose);
    }
}
