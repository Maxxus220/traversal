use clap::Parser;
use const_format::formatcp;
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder, Sink};
use ignore::{WalkBuilder, WalkState};
use regex::Regex;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

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
    tag_list: TagList,
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
        let bytes = mat.bytes();
        let line = std::str::from_utf8(bytes).unwrap_or("");

        let regex = Regex::new(REGEX).expect("Failed to create regex.");
        if let Some(captures) = regex.captures(line) {
            if let Some(group) = captures.get(RegexGroup::TARGET as usize) {
                self.tag_list.targets.write().unwrap().push(TagLocation {
                    path: Box::from(self.path),
                    line_number: line_number,
                    line_content: group.as_str().to_string(),
                })
            }
            if let Some(group) = captures.get(RegexGroup::LINK as usize) {
                self.tag_list.links.write().unwrap().push(TagLocation {
                    path: Box::from(self.path),
                    line_number: line_number,
                    line_content: group.as_str().to_string(),
                })
            }
        }

        Ok(true)
    }
}

struct TagLocation {
    path: Box<std::path::Path>,
    line_number: u64,
    line_content: String,
}

#[derive(Clone)]
struct TagList {
    targets: Arc<RwLock<Vec<TagLocation>>>,
    links: Arc<RwLock<Vec<TagLocation>>>,
}

impl TagList {
    fn new() -> TagList {
        TagList {
            targets: Arc::new(RwLock::new(vec![])),
            links: Arc::new(RwLock::new(vec![])),
        }
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

    let tag_list = TagList::new();
    let walker = walk_builder.build_parallel();

    // Iterate over all files in provided paths except ignored files
    walker.run(|| {
        let tag_list_copy = tag_list.clone();
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
                Agregator {
                    tag_list: tag_list_copy.clone(),
                    path: entry.path(),
                },
            );

            WalkState::Continue
        })
    });

    println!("{:?}", cli_args);
    for target in tag_list.targets.read().unwrap().iter() {
        println!(
            "[TARGET] {}:{}: {}",
            target.path.display(),
            target.line_number,
            target.line_content
        );
    }
    for link in tag_list.links.read().unwrap().iter() {
        println!(
            "[LINK] {}:{}: {}",
            link.path.display(),
            link.line_number,
            link.line_content
        );
    }
}
