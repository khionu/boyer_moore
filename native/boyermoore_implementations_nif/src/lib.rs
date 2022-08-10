use std::collections::HashMap;

use rustler::{Env, NifStruct, ResourceArc};

// Sorting/spacing around imports is varied, but my personal way, with a lineskip between each:
// 1. Core, Alloc, Std
// 2. Imported crates
// 3. Module declarations
// 4. Same-crate imports

#[derive(NifStruct)]
#[module = "BoyerMoore.Implemenations.Nif.BadMatchTable"]
struct BadMatchTable {
    mapping: HashMap<u8, usize>, // Try a BTreeMap? Might be better perf
    size: usize,
}

#[derive(NifStruct)]
#[module = "BoyerMoore.Implemenations.Nif.Pattern"]
struct Pattern {
    match_table: BadMatchTable,
    pattern_chars: Vec<u8>, // Semantic: you're not using `char`s, naming is a misnomer
    size: usize,
}

impl BadMatchTable {
    fn new(pattern: &str) -> Self {
        let pattern_length = pattern.len();
        let mut skip_mappings = HashMap::new();

        for (i, c) in pattern.bytes().enumerate() {
            let skip = pattern_length - i - 1;
            // I personally recommend a new line on either side of an if chain, unless it's at the start/end
            // of the parent block.
            if i == pattern_length - 1 && !skip_mappings.contains_key(&c) {
                skip_mappings.insert(c, pattern_length);
            } else if skip <= 0 {
                skip_mappings.insert(c, pattern_length);
            } else {
                skip_mappings.insert(c, skip);
            }
        }

        BadMatchTable { // You can use Self here too, matter of preference
            mapping: skip_mappings,
            size: pattern_length,
        }
    }

    fn get(&self, c: u8) -> usize {
        // Alternatively:
        // *self.mapping.get(&c).unwrap_or(&self.size)

        match self.mapping.get(&c) {
            Some(value) => *value, // Dereference and it will copy the value, more idiomatic
            None => self.size,
        }
    }
}

impl<'a> Pattern { // This lifetime can be removed, as well as below.
    fn compile(pattern: &'a str) -> Self {
        Pattern {
            match_table: BadMatchTable::new(pattern),
            size: pattern.len(),
            // Alternatively:
            // pattern_chars: pattern.as_bytes().to_owned(),
            pattern_chars: pattern.bytes().collect::<Vec<_>>(),
        }
    }

    fn skip_for(&'a self, c: u8) -> usize {
        self.match_table.get(c)
    }

    fn at(&'a self, i: usize) -> u8 {
        // I'd make this function return an Option and move the unwrap to
        // where it's proven to be a safe assumption. In terms of contracts,
        // there is an implicit contract here that the caller has done the
        // check. It may result in more unwraps, but it's easier to audit.
        *self.pattern_chars.get(i).unwrap()
    }
}

pub fn load(env: Env, _: rustler::Term) -> bool {
    rustler::resource!(Pattern, env);
    true
}

#[rustler::nif]
fn contains(haystack: &str, needle: &str) -> bool {
    let pattern = Pattern::compile(needle);
    do_contains(haystack, &pattern)
}

#[rustler::nif]
fn contains_compiled(haystack: &str, pattern: ResourceArc<Pattern>) -> bool {
    do_contains(haystack, &pattern)
}

#[rustler::nif]
fn compile(pattern: &str) -> ResourceArc<Pattern> {
    ResourceArc::new(Pattern::compile(pattern))
}

fn do_contains(haystack: &str, needle: &Pattern) -> bool {
    let starting_index = needle.size - 1;
    let haystack_chars = haystack.bytes().collect::<Vec<_>>(); // Again, as_bytes
    // Again, may be more personal preference than consensus, but I also put a newline
    // after "prep work" and the rest of a function, even if it's just one line
    contains_pattern(&haystack_chars, needle, starting_index)
}

fn contains_pattern(haystack: &Vec<u8>, pattern: &Pattern, starting_index: usize) -> bool {
    match detect_pattern(haystack, pattern, starting_index, pattern.size - 1) {
        Ok(_) => true,
        Err(skip) if skip + starting_index >= haystack.len() => false,
        Err(skip) =>
            contains_pattern(haystack, pattern, skip + starting_index),
    }
}

fn detect_pattern<'a>( // Can get rid of this lifetime as well
    haystack: &Vec<u8>,
    pattern: &Pattern,
    corpus_index: usize,
    pattern_index: usize,
) -> Result<bool, usize> {
    let haystack_char = *haystack.get(corpus_index).unwrap();
    let pattern_char = pattern.at(pattern_index);

    if haystack_char == pattern_char {
        if pattern_index == 0 {
            Ok(true)
        } else {
            detect_pattern(haystack, pattern, corpus_index - 1, pattern_index - 1)
        }
    } else {
        let skip = pattern.skip_for(haystack_char);
        Err(skip)
    }
}

rustler::init!(
    "Elixir.BoyerMoore.Implementations.Nif",
    [contains, compile, contains_compiled],
    load = load
);
