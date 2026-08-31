use std::collections::HashSet;

use crate::utils::constants::{MAX_KEYWORD_CANDIDATES, MAX_SEARCH_TAGS, MAX_TAG_WORDS};

pub(crate) fn extract_keyword_candidates(text: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current_segment = Vec::new();

    for raw_token in
        text.split(|character: char| !(character.is_alphanumeric() || character == '-'))
    {
        let token = raw_token.trim_matches('-').to_lowercase();
        if token.is_empty() {
            continue;
        }

        if is_phrase_boundary(&token) || token.len() < 3 || !token.chars().any(char::is_alphabetic)
        {
            if !current_segment.is_empty() {
                segments.push(std::mem::take(&mut current_segment));
            }
            continue;
        }

        current_segment.push(token);
    }

    if !current_segment.is_empty() {
        segments.push(current_segment);
    }

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for segment in segments {
        for start in 0..segment.len() {
            let maximum_length = MAX_TAG_WORDS.min(segment.len() - start);
            for phrase_length in (1..=maximum_length).rev() {
                let candidate = segment[start..start + phrase_length].join(" ");
                if is_useful_candidate(&candidate) && seen.insert(candidate.clone()) {
                    candidates.push(candidate);
                    if candidates.len() == MAX_KEYWORD_CANDIDATES {
                        return candidates;
                    }
                }
            }
        }
    }
    candidates
}

pub(crate) fn select_search_tags(mut scored_candidates: Vec<(String, f32)>) -> Vec<String> {
    scored_candidates.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| {
                right
                    .0
                    .split_whitespace()
                    .count()
                    .cmp(&left.0.split_whitespace().count())
            })
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut tags: Vec<String> = Vec::new();
    for (candidate, _) in scored_candidates {
        if tags
            .iter()
            .any(|selected| phrases_are_redundant(selected, &candidate))
        {
            continue;
        }
        tags.push(candidate);
        if tags.len() == MAX_SEARCH_TAGS {
            break;
        }
    }
    tags
}

fn is_phrase_boundary(word: &str) -> bool {
    matches!(
        word,
        "a" | "an"
            | "the"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "from"
            | "of"
            | "with"
            | "without"
            | "near"
            | "beside"
            | "behind"
            | "under"
            | "over"
            | "through"
            | "around"
            | "into"
            | "next"
            | "between"
            | "while"
            | "this"
            | "that"
            | "these"
            | "those"
            | "there"
            | "its"
            | "their"
            | "them"
            | "both"
            | "all"
            | "his"
            | "her"
            | "hers"
            | "same"
            | "other"
            | "image"
            | "photo"
            | "photograph"
            | "picture"
            | "scene"
            | "show"
            | "shows"
            | "showing"
            | "depict"
            | "depicts"
            | "depicting"
            | "feature"
            | "features"
            | "featuring"
            | "camera"
            | "directly"
            | "towards"
            | "front"
            | "top"
            | "right"
            | "left"
            | "middle"
            | "outside"
            | "above"
            | "background"
            | "is"
            | "are"
            | "was"
            | "were"
            | "has"
            | "have"
            | "had"
            | "located"
            | "placed"
            | "displayed"
            | "surrounded"
            | "contains"
            | "including"
    )
}

fn is_useful_candidate(candidate: &str) -> bool {
    let words = candidate.split_whitespace().collect::<Vec<_>>();
    if words.len() == 1 && is_low_information_singleton(words[0]) {
        return false;
    }

    if words.len() > 1
        && words.iter().all(|word| is_descriptor(word))
        && !words.iter().all(|word| is_color_term(word))
    {
        return false;
    }

    true
}

fn is_low_information_singleton(word: &str) -> bool {
    matches!(
        word,
        "open"
            | "closed"
            | "pointed"
            | "pointy"
            | "sticking"
            | "wrinkled"
            | "shown"
            | "together"
            | "large"
            | "small"
            | "long"
            | "short"
            | "tall"
    )
}

fn is_descriptor(word: &str) -> bool {
    is_color_term(word)
        || matches!(
            word,
            "one"
                | "two"
                | "three"
                | "four"
                | "many"
                | "few"
                | "large"
                | "small"
                | "long"
                | "short"
                | "tall"
                | "big"
                | "very"
                | "bit"
                | "colorful"
                | "blurry"
                | "blurred"
                | "pointed"
                | "pointy"
                | "open"
                | "closed"
                | "wrinkled"
                | "sticking"
        )
}

fn is_color_term(word: &str) -> bool {
    matches!(
        word,
        "black"
            | "white"
            | "blue"
            | "green"
            | "red"
            | "brown"
            | "gray"
            | "grey"
            | "yellow"
            | "orange"
            | "pink"
            | "purple"
            | "golden"
            | "light"
            | "dark"
            | "darker"
    )
}

fn phrases_are_redundant(left: &str, right: &str) -> bool {
    let left = left.split_whitespace().collect::<Vec<_>>();
    let right = right.split_whitespace().collect::<Vec<_>>();
    contains_word_sequence(&left, &right) || contains_word_sequence(&right, &left)
}

fn contains_word_sequence(longer: &[&str], shorter: &[&str]) -> bool {
    !shorter.is_empty()
        && longer.len() >= shorter.len()
        && longer
            .windows(shorter.len())
            .any(|window| window == shorter)
}
