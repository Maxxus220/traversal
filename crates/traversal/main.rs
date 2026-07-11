use clap::Parser;
use const_format::formatcp;
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder, Sink};
use ignore::{WalkBuilder, WalkState};
use regex::Regex;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, RwLock};

macro_rules! make_traverse_tag_regex {
    ($tag:expr) => {
        formatcp!(r"\[traverse-{TAG_NAME}:\s*(\S*)\s*\]", TAG_NAME = $tag)
    };
}

const TARGET_TAG_REGEX: &'static str = make_traverse_tag_regex!("tgt");
const LINK_TAG_REGEX: &'static str = make_traverse_tag_regex!("lnk");
const REGEX_STR: &'static str = formatcp!("{TARGET_TAG_REGEX}|{LINK_TAG_REGEX}");
static REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(REGEX_STR).expect("Failed to create regex"));

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
    tag_list: &'a mut TagList,
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

        if let Some(captures) = REGEX.captures(line) {
            if let Some(group) = captures.get(RegexGroup::TARGET as usize) {
                self.tag_list.targets.push(TagLocation {
                    path: Box::from(self.path),
                    line_number: line_number,
                    line_content: group.as_str().to_string(),
                })
            }
            if let Some(group) = captures.get(RegexGroup::LINK as usize) {
                self.tag_list.links.push(TagLocation {
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

// TODO(mfeist): We should be hashing on tag names instead of doing big vectors. Main use case will
// be searching for a tag name, so we should optimize that.
struct TagList {
    targets: Vec<TagLocation>,
    links: Vec<TagLocation>,
}

struct CombinedTagList {
    tag_lists: Vec<TagList>,
}

struct ThreadBuffer {
    tag_list: TagList,
    combined: Arc<RwLock<CombinedTagList>>,
}

impl Drop for ThreadBuffer {
    fn drop(&mut self) {
        let tag_list = TagList {
            targets: std::mem::take(&mut self.tag_list.targets),
            links: std::mem::take(&mut self.tag_list.links),
        };
        self.combined.write().unwrap().tag_lists.push(tag_list);
    }
}

fn main() {
    let cli_args = CliArgs::parse();
    let matcher = Arc::new(
        RegexMatcher::new_line_matcher(REGEX_STR).expect("Failed to create RegexMatcher."),
    );

    let mut walk_builder = WalkBuilder::new(&cli_args.paths[0]);
    for path in &cli_args.paths[1..] {
        walk_builder.add(path);
    }

    let combined_tag_list = Arc::new(RwLock::new(CombinedTagList { tag_lists: vec![] }));
    let walker = walk_builder.build_parallel();

    // Iterate over all files in provided paths except ignored files
    walker.run(|| {
        let matcher_copy = Arc::clone(&matcher);
        let mut searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_number(true)
            .build();
        let mut buffer = ThreadBuffer {
            tag_list: TagList {
                targets: vec![],
                links: vec![],
            },
            combined: combined_tag_list.clone(),
        };
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

            let agregator = Agregator {
                tag_list: &mut buffer.tag_list,
                path: entry.path(),
            };
            let _search_result =
                searcher.search_path(matcher_copy.as_ref(), entry.path(), agregator);

            WalkState::Continue
        })
    });

    // Display tags
    println!("{:?}", cli_args);
    for tag_list in &combined_tag_list.read().unwrap().tag_lists {
        for target in &tag_list.targets {
            println!(
                "[TARGET] {}:{}: {}",
                target.path.display(),
                target.line_number,
                target.line_content
            );
        }
    }
    for tag_list in &combined_tag_list.read().unwrap().tag_lists {
        for link in &tag_list.links {
            println!(
                "[LINK] {}:{}: {}",
                link.path.display(),
                link.line_number,
                link.line_content
            );
        }
    }
}
