use std::{fs, path::PathBuf};

use clap::Parser;

mod class;
mod class_loader;
mod virtual_machine;

#[derive(Parser)]
struct Args {
    filename: PathBuf,
    #[clap(short, long)]
    run: bool,
}

fn main() {
    let args = Args::parse();
    let bytes = fs::read(args.filename).unwrap();
    // let bytes = [
    //     0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 0, 0, 3, 1, 0, 2, 0x30, 0x30, 3, 0, 0, 0, 0xFF, 0, 0, 0,
    //     1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // ];
    let class = class_loader::load_class(&mut bytes.into_iter()).unwrap();
    println!("{class:#?}");
    if args.run {
        virtual_machine::start_vm(class);
    }
}
