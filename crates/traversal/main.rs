use clap::Parser;
use const_format::formatcp;
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder, Sink};
use ignore::{WalkBuilder, WalkState};
use regex::Regex;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

macro_rules! make_traverse_tag_regex {
    ($tag:expr) => {
        formatcp!(r"\[traverse-{TAG_NAME}:\s*(\S*)\s*\]", TAG_NAME = $tag)
    };
}

const TARGET_TAG_REGEX: &'static str = make_traverse_tag_regex!("tgt");
const LINK_TAG_REGEX: &'static str = make_traverse_tag_regex!("lnk");
const REGEX: &'static str = formatcp!("{TARGET_TAG_REGEX}|{LINK_TAG_REGEX}");

enum RegexGroup {
    TARGET = 1,
    LINK = 2,
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<PathBuf>,
}

struct Agregator<'a> {
    path: &'a std::path::Path,
}

impl<'a> Sink for Agregator<'a> {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &grep::searcher::Searcher,
        mat: &grep::searcher::SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        let line_number = mat.line_number().unwrap_or(0);
        let path = self.path.display();
        let bytes = mat.bytes();
        let line = std::str::from_utf8(bytes).unwrap_or("");

        let regex = Regex::new(REGEX).expect("Failed to create regex.");
        if let Some(captures) = regex.captures(line) {
            if let Some(group) = captures.get(RegexGroup::TARGET as usize) {
                println!("[TARGET] {}:{}: {}", path, line_number, group.as_str());
            }
            if let Some(group) = captures.get(RegexGroup::LINK as usize) {
                println!("[LINK] {}:{}: {}", path, line_number, group.as_str());
            }
        }

        Ok(true)
    }
}

fn main() {
    let cli_args = CliArgs::parse();
    let matcher =
        Arc::new(RegexMatcher::new_line_matcher(REGEX).expect("Failed to create RegexMatcher."));

    let mut walk_builder = WalkBuilder::new(&cli_args.paths[0]);
    for path in &cli_args.paths[1..] {
        walk_builder.add(path);
    }

    let walker = walk_builder.build_parallel();

    // Iterate over all files in provided paths except ignored files
    walker.run(|| {
        let matcher_copy = Arc::clone(&matcher);
        let mut searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(true)
            .build();
        Box::new(move |result| {
            let entry = match result {
                Ok(ent) => ent,
                Err(err) => {
                    eprintln!("Error walking directory: {}", err);
                    return WalkState::Continue;
                }
            };

            if !entry.file_type().unwrap().is_file() {
                return WalkState::Continue;
            }

            let _search_result = searcher.search_path(
                matcher_copy.as_ref(),
                entry.path(),
                Agregator { path: entry.path() },
            );

            WalkState::Continue
        })
    });

    println!("{:?}", cli_args);
}
