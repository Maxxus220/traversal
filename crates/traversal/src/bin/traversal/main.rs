use clap::Parser;
use std::path::PathBuf;
use traversal_core::find_tags;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<PathBuf>,
}

fn main() {
    let cli_args = CliArgs::parse();

    let combined_tag_list = find_tags(cli_args.paths);

    // Display tags
    for tag_list in &combined_tag_list.read().unwrap().tag_lists {
        for locations in tag_list.targets.values() {
            for target in locations {
                println!(
                    "[TARGET] {}:{}: {}",
                    target.path.display(),
                    target.line_number,
                    target.line_content
                );
            }
        }
    }
    for tag_list in &combined_tag_list.read().unwrap().tag_lists {
        for locations in tag_list.links.values() {
            for link in locations {
                println!(
                    "[LINK] {}:{}: {}",
                    link.path.display(),
                    link.line_number,
                    link.line_content
                );
            }
        }
    }
}
