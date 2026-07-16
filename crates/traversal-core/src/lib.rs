use const_format::formatcp;
use grep::regex::RegexMatcher;
use grep::searcher::{BinaryDetection, SearcherBuilder, Sink};
use ignore::{WalkBuilder, WalkState};
use regex::Regex;
use std::collections::HashMap;
use std::io;
use std::path::Path;
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

struct Agregator<'a> {
    tag_list: &'a mut TagList,
    path: &'a Path,
}

impl<'a> Sink for Agregator<'a> {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &grep::searcher::Searcher,
        mat: &grep::searcher::SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        // TODO(mfeist): Do a manual byte search with help from memchr::memmem instead for a speed
        // up.
        let line_number = mat.line_number().unwrap_or(0);
        let bytes = mat.bytes();
        let line = std::str::from_utf8(bytes).unwrap_or("");

        if let Some(captures) = REGEX.captures(line) {
            if let Some(group) = captures.get(RegexGroup::TARGET as usize) {
                let tag_name = group.as_str().to_string();
                self.tag_list
                    .targets
                    .entry(tag_name.clone())
                    .or_default()
                    .push(TagLocation {
                        path: Box::from(self.path),
                        line_number: line_number,
                        line_content: tag_name,
                    })
            }
            if let Some(group) = captures.get(RegexGroup::LINK as usize) {
                let tag_name = group.as_str().to_string();
                self.tag_list
                    .links
                    .entry(tag_name.clone())
                    .or_default()
                    .push(TagLocation {
                        path: Box::from(self.path),
                        line_number: line_number,
                        line_content: tag_name,
                    })
            }
        }

        Ok(true)
    }
}

pub struct TagLocation {
    pub path: Box<Path>,
    pub line_number: u64,
    pub line_content: String,
}

pub struct TagList {
    pub targets: HashMap<String, Vec<TagLocation>>,
    pub links: HashMap<String, Vec<TagLocation>>,
}

pub struct CombinedTagList {
    pub tag_lists: Vec<TagList>,
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

pub fn find_tags(
    paths: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Arc<RwLock<CombinedTagList>> {
    let matcher = Arc::new(
        RegexMatcher::new_line_matcher(REGEX_STR).expect("Failed to create RegexMatcher."),
    );

    let mut paths_iter = paths.into_iter();
    let mut walk_builder = WalkBuilder::new(
        paths_iter
            .next()
            .expect("Expected paths to have at least one item."),
    );
    for path in paths_iter {
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
                targets: HashMap::new(),
                links: HashMap::new(),
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

    combined_tag_list
}
