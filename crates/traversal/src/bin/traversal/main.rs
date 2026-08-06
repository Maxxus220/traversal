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
        for target in &tag_list.target_tags {
            println!(
                "[TARGET] {}:{}: {}",
                target.file_path.display(),
                target.line_number,
                target.id
            );
        }
        for link in &tag_list.link_tags {
            println!(
                "[LINK] {}:{}: {}",
                link.file_path.display(),
                link.line_number,
                link.id
            );
        }
    }
}
