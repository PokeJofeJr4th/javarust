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
pub mod data;
pub mod virtual_machine;

#[derive(Parser)]
struct Args {
    filenames: Vec<PathBuf>,
    #[clap(short, long)]
    run: bool,
    #[clap(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    let mut firstclass = None;
    let (mut method_area, mut class_area, heap) = class_loader::load_environment();
    for filename in args.filenames {
        let bytes = fs::read(filename).unwrap();
        // let bytes = [
        //     0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 0, 0, 3, 1, 0, 2, 0x30, 0x30, 3, 0, 0, 0, 0xFF, 0, 0, 0,
        //     1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // ];
        let class = class_loader::load_class(
            &mut method_area,
            &mut class_area,
            &mut bytes.into_iter(),
            args.verbose,
        )
        .unwrap();
        if args.verbose {
            println!("{class:#?}");
        }
        if firstclass.is_none() {
            firstclass = Some(class);
        }
    }
    let Some(class) = firstclass else {
        println!("Error: no class specified");
        return;
    };
    let method_area = method_area.to_shared();
    let class_area = class_area.to_shared();
    let heap = heap.make_shared();
    if args.run {
        virtual_machine::start_vm(class, method_area, class_area, heap, args.verbose);
    }
}
