//! Where the cursor is within a column annotation (`@{…}`).
//!
//! Querydown passes annotations through without interpreting them, so it can't say what keys or
//! values one may contain. It can say where the cursor is (at a key or a value, and along which
//! path of keys and array indices), which lets an application that knows its annotations' structure
//! complete them.
//!
//! Annotations being typed are rarely complete, so rather than parsing one, this scans its text up
//! to the cursor, keeping track of the objects and arrays that are open.

use querydown_parser::highlight::{Token, TokenKind};
use serde::Serialize;

/// Where the cursor is within a column annotation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnnotationPosition {
    /// The keys and array indices leading from the annotation's top-level object to the object or
    /// array the cursor is in, e.g. `["formattingConditions", 0]` within
    /// `@{formattingConditions:[{…}]}`. Empty within the top-level object itself.
    pub path: Vec<AnnotationPathSegment>,
    /// Whether a key or a value is being typed.
    pub slot: AnnotationSlot,
    /// When a key is being typed, the keys the object already has before the cursor.
    pub keys: Vec<String>,
}

/// One step into an annotation: an object's key, or an array's index. Serialized as a plain string
/// or number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum AnnotationPathSegment {
    Key(String),
    Index(usize),
}

/// What is being typed in an annotation. When it's a value, the last segment of the
/// [path](AnnotationPosition::path) says which: the key it belongs to, or its index in an array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnnotationSlot {
    Key,
    Value,
}

/// If `word_start` is within an annotation, where: `Some(Some(position))` at a key or value,
/// `Some(None)` where nothing can be typed (e.g. within a string, or between a key and its `:`).
/// `None` if it isn't within an annotation at all.
pub(super) fn position(
    source: &str,
    word_start: usize,
    tokens: &[Token],
) -> Option<Option<AnnotationPosition>> {
    // An annotation can't contain another, so only the last one begun before the cursor can contain
    // it. The `@` of an annotation is always followed directly by its `{`.
    let at = tokens
        .iter()
        .take_while(|token| token.span.end <= word_start)
        .filter(|token| token.kind == TokenKind::Annotation)
        .last()?;
    // A value being typed as `@true` and so on is completed together with its `@`.
    let end = word_start - usize::from(source[..word_start].ends_with('@'));
    let text = source.get(at.span.end..end)?;
    if !text.starts_with('{') {
        return None;
    }
    scan(text)
}

enum Frame {
    Object {
        segment: Option<AnnotationPathSegment>,
        keys: Vec<String>,
        state: ObjectState,
    },
    Array {
        segment: Option<AnnotationPathSegment>,
        index: usize,
    },
}

enum ObjectState {
    /// Expecting a key (or the end of the object).
    Key,
    /// After a key, expecting its `:`.
    Colon(String),
    /// After a key's `:`, expecting its value.
    Value(String),
}

impl Frame {
    /// The path segment for a value that would begin now within this frame.
    fn child_segment(&self) -> Option<AnnotationPathSegment> {
        match self {
            Frame::Object {
                state: ObjectState::Value(key),
                ..
            } => Some(AnnotationPathSegment::Key(key.clone())),
            Frame::Array { index, .. } => Some(AnnotationPathSegment::Index(*index)),
            _ => None,
        }
    }

    /// Records that a value (or key, in an object expecting one) has ended.
    fn complete(&mut self, word: Option<String>) {
        match self {
            Frame::Object { keys, state, .. } => {
                *state = match std::mem::replace(state, ObjectState::Key) {
                    ObjectState::Key => match word {
                        Some(key) => ObjectState::Colon(key),
                        None => ObjectState::Key,
                    },
                    ObjectState::Value(key) => {
                        keys.push(key);
                        ObjectState::Key
                    }
                    // A value where a `:` was expected: invalid, so start afresh.
                    ObjectState::Colon(_) => ObjectState::Key,
                }
            }
            Frame::Array { index, .. } => *index += 1,
        }
    }
}

/// Scans `text` (an annotation from its opening `{` to the cursor). See [`position`].
fn scan(text: &str) -> Option<Option<AnnotationPosition>> {
    let mut stack: Vec<Frame> = vec![];
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '{' | '[' => {
                let segment = stack.last().and_then(Frame::child_segment);
                stack.push(match c {
                    '{' => Frame::Object {
                        segment,
                        keys: vec![],
                        state: ObjectState::Key,
                    },
                    _ => Frame::Array { segment, index: 0 },
                });
            }
            '}' | ']' => {
                stack.pop();
                match stack.last_mut() {
                    Some(parent) => parent.complete(None),
                    // The annotation ends before the cursor.
                    None => return None,
                }
            }
            ':' => {
                if let Some(Frame::Object { state, .. }) = stack.last_mut() {
                    if let ObjectState::Colon(key) = state {
                        *state = ObjectState::Value(std::mem::take(key));
                    }
                }
            }
            quote @ ('"' | '\'') => {
                let mut value = String::new();
                let mut closed = false;
                while let Some((_, c)) = chars.next() {
                    match c {
                        '\\' => {
                            if let Some((_, escaped)) = chars.next() {
                                value.push(escaped);
                            }
                        }
                        c if c == quote => {
                            closed = true;
                            break;
                        }
                        c => value.push(c),
                    }
                }
                if !closed {
                    return Some(None);
                }
                stack.last_mut()?.complete(Some(value));
            }
            '/' if text[i..].starts_with("//") => {
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '/' if text[i..].starts_with("/*") => {
                let Some(end) = text[i + 2..].find("*/") else {
                    return Some(None);
                };
                while chars.peek().is_some_and(|(j, _)| *j < i + 2 + end + 2) {
                    chars.next();
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                // A bare word, number or `@` constant, which runs until whitespace, punctuation or
                // a bracket.
                let mut word = c.to_string();
                while let Some((_, c)) = chars.peek() {
                    if c.is_whitespace() || "{}[]:\"'".contains(*c) {
                        break;
                    }
                    word.push(*c);
                    chars.next();
                }
                stack.last_mut()?.complete(Some(word));
            }
        }
    }
    let frame = stack.last()?;
    let mut path: Vec<AnnotationPathSegment> = stack
        .iter()
        .filter_map(|frame| match frame {
            Frame::Object { segment, .. } | Frame::Array { segment, .. } => segment.clone(),
        })
        .collect();
    let position = match frame {
        Frame::Object {
            keys,
            state: ObjectState::Key,
            ..
        } => AnnotationPosition {
            path,
            slot: AnnotationSlot::Key,
            keys: keys.clone(),
        },
        Frame::Object {
            state: ObjectState::Colon(_),
            ..
        } => return Some(None),
        Frame::Object { .. } | Frame::Array { .. } => {
            path.extend(frame.child_segment());
            AnnotationPosition {
                path,
                slot: AnnotationSlot::Value,
                keys: vec![],
            }
        }
    };
    Some(Some(position))
}
