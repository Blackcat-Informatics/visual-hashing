// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conformance against the frozen vectors (the Python reference is the oracle).
//! The standalone repository keeps the reference vectors under `vectors/`.
//!
//! The vectors are read by the small strict parser below rather than by a JSON
//! crate. They have a fixed, generated shape — flat objects of strings,
//! non-negative integers and integer arrays — so the parser stays short, and
//! every departure from that shape is a hard error rather than a shrug. A
//! lenient reader is the real hazard here: a vector that silently parses to
//! the wrong thing would weaken exactly the check this file exists to make.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use visual_hashing::{emoji_indices, emojihash, emojihash_labels, randomart};

fn vectors_dir(kind: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("vectors")
        .join(kind);
    assert!(dir.is_dir(), "missing required vector corpus: {dir:?}");
    dir
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
        .collect()
}

// --- the strict vector reader -------------------------------------------------

#[derive(Debug)]
enum Value {
    Str(String),
    Uint(u64),
    Uints(Vec<u64>),
}

impl Value {
    fn str(&self) -> &str {
        match self {
            Value::Str(s) => s,
            other => panic!("expected a string, found {other:?}"),
        }
    }

    fn uint(&self) -> u64 {
        match self {
            Value::Uint(n) => *n,
            other => panic!("expected an integer, found {other:?}"),
        }
    }

    fn uints(&self) -> &[u64] {
        match self {
            Value::Uints(v) => v,
            other => panic!("expected an integer array, found {other:?}"),
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, pos: 0 }
    }

    fn fail(&self, what: &str) -> ! {
        panic!("malformed vector at byte {}: {what}", self.pos);
    }

    fn skip_ws(&mut self) {
        while matches!(self.bytes.get(self.pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, byte: u8) {
        self.skip_ws();
        if self.bytes.get(self.pos) != Some(&byte) {
            self.fail(&format!("expected {:?}", byte as char));
        }
        self.pos += 1;
    }

    fn peek(&mut self) -> u8 {
        self.skip_ws();
        match self.bytes.get(self.pos) {
            Some(&b) => b,
            None => self.fail("unexpected end of input"),
        }
    }

    /// Multi-byte UTF-8 needs no special handling: its continuation bytes are
    /// never `"` or `\`, so accumulating raw bytes and validating once at the
    /// close quote is both simpler and stricter than decoding as we go.
    fn string(&mut self) -> String {
        self.eat(b'"');
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = match self.bytes.get(self.pos) {
                Some(&b) => b,
                None => self.fail("unterminated string"),
            };
            self.pos += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(out)
                        .unwrap_or_else(|_| panic!("invalid UTF-8 in vector string"))
                }
                b'\\' => {
                    let escape = match self.bytes.get(self.pos) {
                        Some(&b) => b,
                        None => self.fail("unterminated escape"),
                    };
                    self.pos += 1;
                    out.push(match escape {
                        b'"' => b'"',
                        b'\\' => b'\\',
                        b'/' => b'/',
                        b'b' => 0x08,
                        b'f' => 0x0c,
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        // The generators write UTF-8 directly (`ensure_ascii=False`),
                        // so a `\u` escape means the corpus format changed and this
                        // reader must be revisited rather than guessed at.
                        _ => self.fail("unsupported escape; extend the reader"),
                    });
                }
                _ => out.push(byte),
            }
        }
    }

    fn uint(&mut self) -> u64 {
        self.skip_ws();
        let start = self.pos;
        while matches!(self.bytes.get(self.pos), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        if start == self.pos {
            self.fail("expected a non-negative integer");
        }
        std::str::from_utf8(&self.bytes[start..self.pos])
            .expect("ascii digits")
            .parse()
            .unwrap_or_else(|_| panic!("integer out of range in vector"))
    }

    fn value(&mut self) -> Value {
        match self.peek() {
            b'"' => Value::Str(self.string()),
            b'[' => {
                self.eat(b'[');
                let mut items = Vec::new();
                if self.peek() == b']' {
                    self.eat(b']');
                    return Value::Uints(items);
                }
                loop {
                    items.push(self.uint());
                    match self.peek() {
                        b',' => self.eat(b','),
                        b']' => {
                            self.eat(b']');
                            return Value::Uints(items);
                        }
                        _ => self.fail("expected ',' or ']'"),
                    }
                }
            }
            b'0'..=b'9' => Value::Uint(self.uint()),
            _ => self.fail("vectors hold only strings, integers and integer arrays"),
        }
    }

    fn object(&mut self) -> BTreeMap<String, Value> {
        let mut fields = BTreeMap::new();
        self.eat(b'{');
        if self.peek() == b'}' {
            self.eat(b'}');
            return fields;
        }
        loop {
            let key = self.string();
            self.eat(b':');
            let value = self.value();
            if fields.insert(key.clone(), value).is_some() {
                panic!("duplicate key {key:?} in vector");
            }
            match self.peek() {
                b',' => self.eat(b','),
                b'}' => {
                    self.eat(b'}');
                    return fields;
                }
                _ => self.fail("expected ',' or '}'"),
            }
        }
    }
}

/// Read one vector, insisting on exactly `expected` keys.
///
/// The exact-key-set assertion is what makes a hand-written reader safe to
/// rely on: a renamed, dropped or added field fails loudly instead of leaving
/// an assertion silently unexercised.
fn read_vector(path: &Path, expected: &[&str]) -> BTreeMap<String, Value> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("reading {path:?}: {e}"));
    let mut reader = Reader::new(&bytes);
    let fields = reader.object();
    reader.skip_ws();
    assert_eq!(reader.pos, bytes.len(), "trailing content in {path:?}");

    let found: Vec<&str> = fields.keys().map(String::as_str).collect();
    let mut want: Vec<&str> = expected.to_vec();
    want.sort_unstable();
    assert_eq!(found, want, "unexpected field set in {path:?}");

    fields
}

fn vector_files(kind: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(vectors_dir(kind))
        .unwrap_or_else(|e| panic!("vectors/{kind} must exist: {e}"))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
}

// --- the conformance checks ---------------------------------------------------

#[test]
fn emojihash_vectors() {
    let paths = vector_files("emojihash");
    for path in &paths {
        let v = read_vector(path, &["data", "emoji", "indices", "labels", "length"]);
        let data = unhex(v["data"].str());
        let length = v["length"].uint() as usize;
        let want_indices: Vec<usize> = v["indices"].uints().iter().map(|&i| i as usize).collect();

        assert_eq!(emoji_indices(&data, length), want_indices, "{path:?}");
        assert_eq!(emojihash(&data, length), v["emoji"].str(), "{path:?}");
        assert_eq!(
            emojihash_labels(&data, length),
            v["labels"].str(),
            "{path:?}"
        );
    }
    assert!(
        paths.len() >= 4,
        "expected emojihash vectors, found {}",
        paths.len()
    );
}

#[test]
fn randomart_vectors() {
    let paths = vector_files("randomart");
    for path in &paths {
        let v = read_vector(path, &["art", "data", "label"]);
        let data = unhex(v["data"].str());
        assert_eq!(
            randomart(&data, v["label"].str()),
            v["art"].str(),
            "{path:?}"
        );
    }
    assert!(
        paths.len() >= 5,
        "expected randomart vectors, found {}",
        paths.len()
    );
}

#[test]
fn reader_rejects_malformed_vectors() {
    let cases: &[(&str, &str)] = &[
        ("{\"a\": 1,}", "trailing comma"),
        ("{\"a\": 1} junk", "trailing content"),
        ("{\"a\": -1}", "negative number"),
        ("{\"a\": 1.5}", "float"),
        ("{\"a\": true}", "boolean"),
        ("{\"a\": [1, \"x\"]}", "mixed array"),
        ("{\"a\": 1, \"a\": 2}", "duplicate key"),
        ("{\"a\": \"\\u0041\"}", "unicode escape"),
        ("{\"a\": \"unterminated}", "unterminated string"),
    ];

    // These cases panic by design; keep the expected noise out of the output.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    for (input, what) in cases {
        let result = std::panic::catch_unwind(|| {
            let bytes = input.as_bytes();
            let mut reader = Reader::new(bytes);
            let fields = reader.object();
            reader.skip_ws();
            assert_eq!(reader.pos, bytes.len());
            fields
        });
        assert!(result.is_err(), "reader accepted {what}: {input}");
    }

    std::panic::set_hook(hook);
}
