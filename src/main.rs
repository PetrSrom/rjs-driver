use clap::{Parser};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(short='c', long="config", value_name = "PATH", help = "Cesta k config souboru")]
    config_file: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    println!("{:?}", cli);
}
