use std::collections::HashSet;
use std::sync::LazyLock;
use std::{collections::HashMap, fs, num::NonZero, path::Path};

use fancy_regex::{Captures, Regex};
use itertools::Itertools;
use notify_debouncer_mini::DebouncedEvent;
use piper_rs::Piper;
use rodio::{MixerDeviceSink, Player, buffer::SamplesBuffer};
use walkdir::WalkDir;

use crate::components::claude_entry::ClaudeEntry;
use crate::components::config::Config;

pub struct Voice {
    speaker: Option<i64>,
    handle: MixerDeviceSink,
    piper: Piper,
    player: Player,
    files: HashMap<String, usize>,
}

impl Voice {
    pub fn new(config: Config) -> Voice {
        let piper = Piper::new(
            Path::new(&config.model_path),
            Path::new(&config.config_path),
        )
        .unwrap();

        let handle = rodio::DeviceSinkBuilder::open_default_sink().unwrap();
        let player = rodio::Player::connect_new(handle.mixer());
        player.set_speed(1.0);

        let mut files = HashMap::<String, usize>::default();
        for file in WalkDir::new(config.monitored_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let f_path = file.path().to_str().unwrap();
            if f_path.ends_with(".json") {
                dbg!(f_path);
                let last_ix = fs::read_to_string(f_path)
                    .unwrap()
                    .lines()
                    .filter(|line| !line.is_empty())
                    .count();
                files.insert(f_path.to_owned(), last_ix);
            }
        }

        Voice {
            speaker: Some(config.speaker_id),
            handle,
            piper,
            player,
            files,
        }
    }

    pub fn wait(&self) {
        self.player.sleep_until_end();
    }

    fn append_from_text(&mut self, text: String) {
        let (samples, sample_rate) = self
            .piper
            .create(&sanitize_text(text), false, self.speaker, None, None, None)
            .unwrap();

        let buffer = SamplesBuffer::new(
            NonZero::new(1).unwrap(),
            NonZero::new(sample_rate).unwrap(),
            samples,
        );

        self.player.append(buffer);
    }

    pub fn append_from_event(&mut self, event: DebouncedEvent) {
        let path = event.path.to_str().unwrap();

        if !path.ends_with(".json") {
            return;
        }

        let last_ix = *self.files.get(path).unwrap_or(&0);
        let file = fs::read_to_string(path).unwrap();
        let messages = file
            .lines()
            .filter(|line| !line.is_empty())
            .enumerate()
            .skip(last_ix)
            .filter_map(|(ix, line)| serde_json::from_str::<ClaudeEntry>(line).ok().zip(Some(ix)))
            .filter(|(entry, _)| {
                entry.r#type == "assistant"
                    && entry.message.r#type == "message"
                    && entry.message.role == "assistant"
            })
            .flat_map(|(entry, ix)| {
                entry
                    .message
                    .content
                    .into_iter()
                    .filter_map(|c| {
                        if c.r#type == "text" {
                            Some((c.text, ix))
                        } else {
                            None
                        }
                    })
                    .collect_vec()
            });

        for (msg, ix) in messages {
            self.append_from_text(msg);
            self.files
                .entry(path.to_owned())
                .and_modify(|x| *x = ix + 1)
                .or_insert(ix + 1);
        }
    }
}

static MARKDOWN_LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^]]+)\]\([^)]+\)").unwrap());
static CONSECUTIVE_DIGITS_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\d{4,}").unwrap());
static CONSECUTIVE_CAPITALS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Z]{3,}").unwrap());

fn sanitize_text(text: String) -> String {
    let text = MARKDOWN_LINK_REGEX
        .replace_all(&text, |caps: &Captures<'_, str>| caps[1].replace('/', " "));

    let text = replace_table(text.to_string());

    let text = text
        .replace(": ", ": \n")
        .replace(['(', ')', '→'], " — ")
        .replace('/', " and ");

    let text = CONSECUTIVE_DIGITS_REGEX.replace_all(&text, |caps: &Captures<'_, str>| {
        caps[0].chars().intersperse(' ').collect::<String>()
    });

    let text = CONSECUTIVE_CAPITALS_REGEX.replace_all(&text, |caps: &Captures<'_, str>| {
        caps[0].chars().intersperse(' ').collect::<String>()
    });

    text.replace("\\n", "\n")
        .replace("\\t", " ")
        .replace("\\", " backslash ")
        .replace(['\\', '*', '#', '>'], "")
        + " — "
}

const TABLE_REPLACEMENT: &str = " — see the attached table — ";

fn replace_table(text: String) -> String {
    let (headers, rows): (Vec<_>, Vec<_>) = text
        .lines()
        .enumerate()
        .map(|(ix, line)| {
            (
                line.replace("\\|", "")
                    .chars()
                    .filter(|c| *c == '|')
                    .count(),
                ix,
            )
        })
        .chunk_by(|(c, _)| *c)
        .into_iter()
        .map(|(k, v)| (k, v.collect_vec()))
        .filter_map(|(k, v)| {
            if k != 0 && v.len() > 1 {
                Some((
                    v.first().unwrap().1,
                    v.into_iter().map(|(_, ix)| ix).collect_vec(),
                ))
            } else {
                None
            }
        })
        .unzip();

    let headers: HashSet<usize> = HashSet::from_iter(headers);
    let rows: HashSet<usize> = HashSet::from_iter(rows.into_iter().flatten());

    text.lines()
        .enumerate()
        .filter_map(|(ix, line)| {
            if headers.contains(&ix) {
                Some(TABLE_REPLACEMENT)
            } else if rows.contains(&ix) {
                None
            } else {
                Some(line)
            }
        })
        .collect()
}
