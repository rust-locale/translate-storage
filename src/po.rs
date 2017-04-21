//! Handling of [Uniforum Portable Objects][PO]
//!
//! This format is used by the well known [gettext] suite and also supported by the
//! [translate-toolkit][tt] suite. It is a simple text format storing translation units with
//! optional context and plural variants.
//!
//! For modern translation work it's disadvantage is the plural system only supports integers.
//!
//! [PO]: https://www.gnu.org/software/gettext/manual/html_node/PO-Files.html
//! [gettext]: https://www.gnu.org/software/gettext/
//! [tt]: http://toolkit.translatehouse.org/

use locale_config::LanguageRange;
use regex::{Regex,Captures};
use std::collections::{BTreeMap,HashMap};
use std::io::{BufRead,Lines};
use std::iter::Peekable;
use super::{CatalogueReader,Count,Error,Message,Origin,State,Unit};

#[derive(Clone,Debug)]
enum PoLine {
    // (number, kind (translator is space), content of the comment)
    Comment(usize, char, String),
    // (number, obsolete/previous flag, tag, string)
    Message(usize, String, String, String),
    // (number, obsolete/previous flag, string)
    Continuation(usize, String, String),
    // ()
    Blank,
}

struct LineIter<R: BufRead> {
    _n: usize,
    _inner: Lines<R>,
}

lazy_static!{
    static ref MESSAGE_RE: Regex = Regex::new(
        r#"^\s*(#~?\|?)?\s*(msgctxt|msgid|msgif_plural|msgstr(?:\[[012345]\])?)?\s*"(.*)"\s*$"#)
        .unwrap();
    static ref COMMENT_RE: Regex = Regex::new(
        r#"^\s*#([:.,]?)\s*(.*)"#).unwrap();

    static ref UNESCAPE_RE: Regex = Regex::new("\\\\[rtn\"\\]").unwrap();
    static ref UNESCAPE_MAP: HashMap<&'static str, &'static str> = [
        (r"\r", "\r"),
        (r"\t", "\t"),
        (r"\n", "\n"),
        ("\\\"", "\""),
        (r"\\", r"\"),
    ].iter().cloned().collect();
}

fn parse_po_line(line: &str, n: usize) -> Result<PoLine, ()> {
    if !line.contains(|c: char| !c.is_whitespace()) {
        return Ok(PoLine::Blank);
    }
    if let Some(c) = MESSAGE_RE.captures(line) {
        if c.get(2).is_some() {
            return Ok(PoLine::Message(
                    n,
                    c.get(1).unwrap().as_str().to_owned(),
                    if c.get(1).unwrap().as_str().ends_with('|') {
                        String::from("|") + c.get(2).unwrap().as_str()
                    } else {
                        c.get(2).unwrap().as_str().to_owned()
                    },
                    UNESCAPE_RE.replace(
                        c.get(3).unwrap().as_str(),
                        |d: &Captures| -> String {
                            UNESCAPE_MAP.get(d.get(0).unwrap().as_str()).unwrap().to_string()
                        }).into_owned()));
        } else {
            return Ok(PoLine::Continuation(
                    n,
                    c.get(1).unwrap().as_str().to_owned(),
                    UNESCAPE_RE.replace(
                        c.get(3).unwrap().as_str(),
                        |d: &Captures| -> String {
                            UNESCAPE_MAP.get(d.get(0).unwrap().as_str()).unwrap().to_string()
                        }).into_owned()));
        }
    }
    if let Some(c) = COMMENT_RE.captures(line) {
        return Ok(PoLine::Comment(
                n,
                c.get(1).unwrap().as_str().chars().next().unwrap_or(' '),
                c.get(2).unwrap().as_str().to_owned()));
    }
    return Err(());
}

impl<R: BufRead> Iterator for LineIter<R> {
    type Item = Result<PoLine, Error>;
    fn next(&mut self) -> Option<Result<PoLine, Error>> {
        loop {
            let line = match self._inner.next() {
                Some(Ok(s)) => s,
                Some(Err(e)) => return Some(Err(Error::Io(self._n + 1, e))),
                None => return None,
            };
            self._n += 1;
            match parse_po_line(&line, self._n) {
                Ok(PoLine::Blank) => (),
                Ok(p) => return Some(Ok(p)),
                Err(_) => return Some(Err(Error::Parse(self._n, Some(line), Vec::new()))),
            }
        }
    }
}

impl<R: BufRead> LineIter<R> {
    fn new(r: R) -> LineIter<R> {
        LineIter {
            _n: 0,
            _inner: r.lines(),
        }
    }
}

trait MsgParser {
    fn parse_comments(&mut self, unit: &mut Unit);
    fn parse_msg(&mut self, tag: &str, unit: &mut Unit) -> Result<Option<String>, Error>;
    fn expected(&mut self, exp: Vec<&'static str>) -> Result<Option<Unit>, Error>;
}

impl<R: BufRead> MsgParser for Peekable<LineIter<R>> {
    fn parse_comments(&mut self, unit: &mut Unit) {
        while let Some(&Ok(PoLine::Comment(..))) = self.peek() {
            match self.next() {
                Some(Ok(PoLine::Comment(_, ',', s))) => {
                    for flag in s.split(',').map(str::trim) {
                        match flag {
                            "fuzzy" => unit._state = State::NeedsWork,
                            _ => (), // TODO: Implement other flags (do we need any?)
                        }
                    }
                }
                Some(Ok(PoLine::Comment(_, ':', s))) => {
                    unit._locations.extend(s.split(",").map(str::trim).map(From::from));
                }
                Some(Ok(PoLine::Comment(_, '.', s))) => {
                    unit._notes.push((Origin::Developer, s));
                }
                Some(Ok(PoLine::Comment(_, ' ', s))) => {
                    unit._notes.push((Origin::Translator, s));
                }
                _ => unreachable!(), // we *know* it is a Some(Ok(Comment))
            }
        }
    }

    fn parse_msg(&mut self, tag: &str, unit: &mut Unit) -> Result<Option<String>, Error> {
        if is!(self.peek() => Some(&Err(_))) {
            // Get error out of the way
            return Err(self.next().unwrap().unwrap_err())
        }
        
        let prefix;
        let mut string;

        if is!(self.peek() =>
               Some(&Ok(PoLine::Message(_, ref p, ref t, _)))
               if t == tag && p.starts_with("#~") == unit._obsolete) {
            if let PoLine::Message(_, p, _, s) = self.next().unwrap().unwrap() {
                prefix = p;
                string = s;
            } else {
                unreachable!()
            }
        } else {
            return Ok(None); // Not the expected message
        }

        loop {
            if is!(self.peek() => Some(&Err(_))) {
                // Get error out of the way
                return Err(self.next().unwrap().unwrap_err())
            }

            if is!(self.peek() =>
                   Some(&Ok(PoLine::Continuation(_, ref p, _)))
                   if *p == prefix) {
                if let PoLine::Continuation(_, _, s) = self.next().unwrap().unwrap() {
                    string.push_str(&s);
                } else {
                    unreachable!();
                }
            } else {
                break;
            }
        }
        Ok(Some(string))
    }

    fn expected(&mut self, exp: Vec<&'static str>) -> Result<Option<Unit>, Error> {
        match self.peek() {
            Some(&Ok(PoLine::Message(n, ref p, ..))) =>
                Err(Error::Parse(n, Some(p.clone()), exp)),
            Some(&Ok(PoLine::Continuation(n, ..))) =>
                Err(Error::Parse(n, Some("\"".to_owned()), exp)),
            Some(&Ok(PoLine::Comment(n, c, ..))) =>
                Err(Error::Parse(n, Some(format!("#{}", c)), exp)),
            None =>
                Ok(None),
            _ => panic!("Should not happen!"),
        }
    }
}

fn is_header(oru: &Option<Result<Unit, Error>>) -> bool {
    match oru {
        &Some(Ok(ref u)) => u.source().is_singular() && u.source().is_blank(),
        _ => false,
    }
}

pub struct PoReader<R: BufRead> {
    _lines: Peekable<LineIter<R>>,
    _next_unit: Option<Result<Unit, Error>>,
    _failed: Option<Error>,
    _header: HashMap<String, String>,
    _target_language: LanguageRange<'static>,
    _plurals: Vec<Count>,
}

impl<R: BufRead> PoReader<R> {
    pub fn new(reader: R) -> Self {
        let mut res = PoReader {
            _lines: LineIter::new(reader).peekable(),
            _next_unit: None,
            _failed: None,
            _header: HashMap::new(),
            _target_language: LanguageRange::invariant(),
            _plurals: Vec::new(),
        };
        res._next_unit = res.next_unit();
        if is_header(&res._next_unit) {
            res.parse_po_header();
            res._next_unit = res.next_unit();
        }
        return res;
    }

    fn make_source(msgid: Option<String>, msgid_plural: Option<String>) -> Message {
        if msgid.is_none() {
            Message::Empty
        } else if msgid_plural.is_none() {
            Message::Singular(msgid.unwrap())
        } else {
            let mut map = BTreeMap::new();
            map.insert(Count::One, msgid.unwrap());
            map.insert(Count::Other, msgid_plural.unwrap());
            Message::Plural(map)
        }
    }

    fn parse_unit(&mut self) -> Result<Option<Unit>, Error> {
        let mut unit = Unit::default();
        self._lines.parse_comments(&mut unit);
        match self._lines.peek() {
            None => return Ok(None), // end if no unit (possibly after comments)
            Some(&Ok(PoLine::Message(_, ref p, ..))) // detect obsolete
                if p.starts_with("#~") => unit._obsolete = true,
            _ => (),
        }

        unit._prev_context = self._lines.parse_msg("|msgctxt", &mut unit)?;

        let prev_msgid = self._lines.parse_msg("|msgid", &mut unit)?;
        let prev_msgid_pl = if prev_msgid.is_some() {
            self._lines.parse_msg("|msgid_plural", &mut unit)?
        } else { None };
        unit._prev_source = Self::make_source(prev_msgid, prev_msgid_pl);

        unit._context = self._lines.parse_msg("msgctxt", &mut unit)?;

        let msgid = self._lines.parse_msg("msgid", &mut unit)?;
        if msgid.is_none() {
            return self._lines.expected(vec!["msgid"]);
        }
        let msgid_pl = self._lines.parse_msg("msgid_plural", &mut unit)?;
        unit._source = Self::make_source(msgid, msgid_pl);

        if unit._source.is_singular() {
            // sinngular source, so expecting singular target:
            match self._lines.parse_msg("msgstr", &mut unit)? {
                None => return self._lines.expected(vec!["msgstr"]),
                Some(s) => unit._target = Message::Singular(s),
            }
        } else {
            assert!(unit._source.is_plural());
            const TAGS: &[&'static str] =
                &["msgstr[0]", "msgstr[1]", "msgstr[2]", "msgstr[3]", "msgstr[4]", "msgstr[5]", "msgstr[6]"];
            let mut map = BTreeMap::new();
            for (c, t) in self._plurals.iter().zip(TAGS) {
                match self._lines.parse_msg(t, &mut unit)? {
                    None => return self._lines.expected(vec![t]),
                    Some(s) => { map.insert(*c, s); }
                }
            }
            unit._target = Message::Plural(map);
        }

        assert!(!unit._source.is_empty());
        return Ok(Some(unit));
    }

    fn next_unit(&mut self) -> Option<Result<Unit, Error>> {
        match self.parse_unit() {
            Ok(None) => None,
            Ok(Some(u)) => Some(Ok(u)),
            Err(e) => Some(Err(e)),
        }
    }

    fn parse_po_header(&mut self) {
        if let Some(Ok(ref u)) = self._next_unit {
            for line in u._target.singular().unwrap_or("").split('\n') {
                if let Some(n) = line.find(':') {
                    let key = line[..n].trim();
                    let val = line[(n+1)..].trim();
                    self._header.insert(key.to_owned(), val.to_owned());
                }
            }
            if let Some(lang) = self._header.get("Language") {
                self._target_language
                    = LanguageRange::new(lang)
                    .map(LanguageRange::into_static)
                    .or_else(|_| LanguageRange::from_unix(lang))
                    .unwrap_or_else(|_| LanguageRange::invariant());
            }
            // FIXME FIXME: Extract plurals
        }
    }
}

impl<R: BufRead> Iterator for PoReader<R> {
    type Item = Result<Unit, Error>;
    fn next(&mut self) -> Option<Result<Unit, Error>> {
        if self._next_unit.is_none() {
            return None;
        }

        let mut res = self.next_unit();
        ::std::mem::swap(&mut res, &mut self._next_unit);
        return res;
    }
}

impl<R: BufRead> CatalogueReader for PoReader<R> {
    fn target_language(&self) -> &LanguageRange<'static> {
        &self._target_language
    }
}
