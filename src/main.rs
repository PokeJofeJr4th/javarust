#![warn(clippy::nursery, clippy::pedantic)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions
)]

use std::{error::Error, fs, path::PathBuf};

use clap::Parser;
use data::Heap;

pub mod class;
pub mod class_loader;
pub mod data;
pub mod virtual_machine;

#[derive(Parser, Debug)]
#[allow(clippy::struct_field_names)]
struct Args {
    /// the filenames of the classes to run. The first filename will be treated as the main class
    filenames: Vec<PathBuf>,
    #[clap(short, long)]
    /// whether to run the main method of the first resolved class
    run: bool,
    #[clap(short, long)]
    verbose: bool,
    /// use this option to read dependencies from a file containing one relative path per line
    #[clap(short, long)]
    project: Option<PathBuf>,
    /// pass these values as arguments to the java program
    #[clap(last = true, allow_hyphen_values = true)]
    program_args: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    if args.verbose {
        println!("{args:?}");
    }
    let mut firstclass = None;
    let (mut method_area, mut class_area) = class_loader::load_environment();
    let mut filenames = args.filenames;
    // include any paths from a project file
    if let Some(projpath) = args.project {
        let projfile = fs::read_to_string(&projpath)?;
        let projpath = projpath.parent().unwrap();
        for line in projfile.lines() {
            filenames.push(projpath.join(line));
        }
    }
    let mut filenames = filenames
        .into_iter()
        .map(|filename| filename.canonicalize())
        .collect::<Result<Vec<_>, _>>()?;
    let first_file = filenames.remove(0);
    filenames.sort();
    filenames.dedup();
    filenames.retain(|p| p != &first_file);
    filenames.insert(0, first_file);
    for filename in filenames {
        if args.verbose {
            println!("Reading class from {filename:?}...");
        }
        let bytes = fs::read(filename)?;
        // let bytes = [
        //     0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 0, 0, 3, 1, 0, 2, 0x30, 0x30, 3, 0, 0, 0, 0xFF, 0, 0, 0,
        //     1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        // ];
        let class =
            class_loader::load_class(&mut method_area, &mut bytes.into_iter(), args.verbose)
                .unwrap();
        if args.verbose {
            println!("{class:#?}");
        }
        if firstclass.is_none() {
            firstclass = Some(class.this.clone());
        }
        class_area.push(class);
    }
    let Some(class) = firstclass else {
        println!("Error: no class specified");
        return Ok(());
    };
    let class_area = class_area.to_shared();
    let method_area = method_area.to_shared(&class_area, args.verbose)?;
    let heap = Heap::new(class_area.clone()).make_shared();
    if args.verbose {
        println!("{method_area:#?}");
    }
    if args.run {
        virtual_machine::start_vm(
            &class,
            method_area,
            class_area,
            heap,
            args.program_args,
            args.verbose,
        );
    }
    Ok(())
}
