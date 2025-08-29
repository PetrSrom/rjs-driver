mod xml_reader;

use clap::Parser;
use xml_reader::XmlData;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(short = 'g', long = "generate_service")]
    generate_service_file: bool,

    #[arg(
        short = 'c',
        long = "config",
        value_name = "PATH",
        help = "Cesta k config souboru"
    )]
    config_file: Option<String>,
}

fn main() {
    let mut cli = Cli::parse();

    if cli.config_file.is_none() {
        cli.config_file = Some("./files/IND15.xml".into());
    }

    let data = XmlData::read_from_xml_file(cli.config_file.as_ref().unwrap())
        .unwrap_or_else(|e| panic!("{e}"));

    for i in data.rjss {
        println!("{:?}", i)
    }

    for i in data.diagnet {
        println!("DlsIP: {:?}", i)
    }
}
