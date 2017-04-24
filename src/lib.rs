//! Translation catalogues are key part of any localization infrastructure. They contain the lists
//! of messages from the application, possibly disambiguated with identifiers or contexts, and
//! corresponding translations.
//!
//! Catalogs are usually stored in one of two formats: [Portable Objects (`.po`)][PO], used
//! primarily by [GNU gettext][gettext], and [XML Localisation Interchange File Format
//! (XLIFF)][XLIFF], a more generic OASIS open standard.
//!
//! These formats can be converted to each other, and to and from many others, using
//! [translate-toolkit][tt].
//!
//! [XLIFF] is quite flexible and can be used in different ways, but this library focuses
//! primarily on using it in a way [gettext] and [translate-toolkit][tt] work, namely with separate
//! catalogue for each language.
//!
//! [PO]: https://www.gnu.org/software/gettext/manual/html_node/PO-Files.html
//! [XLIFF]: https://www.oasis-open.org/committees/xliff/
//! [gettext]: https://www.gnu.org/software/gettext/
//! [tt]: http://toolkit.translatehouse.org/

#[macro_use]
extern crate lazy_static;

extern crate locale_config;

extern crate regex;

use std::collections::BTreeMap;
use locale_config::LanguageRange;

// Auxiliary macros for match checking and then not holding on to the value:
macro_rules! unpack {
    ($x:expr => $p:pat => $r:expr) => {{
        match $x {
            $p => Some($r),
            _ => None,
        }
    }};
    ($x:expr => $p:pat if $c:expr => $r:expr) => {{
        match $x {
            $p if $c => Some($r),
            _ => None,
        }
    }};
}

macro_rules! is {
    ($x:expr => $p:pat) => {{
        match $x {
            $p => true,
            _ => false,
        }
    }};
    ($x:expr => $p:pat if $c:expr) => {{
        match $x {
            $p if $c => true,
            _ => false,
        }
    }};
}


pub mod po;

/// Plural variants
///
/// Which variants are used depends on the language. In English it is easy: 1 is One and everything
/// else is Other. But other languages may have more cases, with Arabic having all six.
// TODO: When Count is in locale, use that version
#[derive(Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub enum Count {
    /// Zero has a separate variant in some langauges.
    Zero,
    /// One. In some languages also includes zero.
    One,
    /// Special case for two.
    Two,
    /// Small number. What is small number depends on the language.
    Few,
    /// Large number. What is large number depends on the language.
    Many,
    /// Any other number.
    Other,
}

impl Default for Count {
    fn default() -> Count { Count::One }
}

/// String wrapper possibly with plural variants.
///
/// This is used for source and target strings in translation Unit.
#[derive(Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub enum Message {
    /// Unset message, used for untranslated entries.
    Empty,
    /// Message independent of any count.
    Singular(String),
    /// Count-dependent message with some variants. Must have at least variant for Other.
    Plural(BTreeMap<Count, String>),
}

impl Message {
    pub fn is_empty(&self) -> bool {
        match self {
            &Message::Empty => true,
            _ => false,
        }
    }

    pub fn is_singular(&self) -> bool {
        match self {
            &Message::Singular(_) => true,
            _ => false,
        }
    }

    pub fn is_plural(&self) -> bool {
        match self {
            &Message::Plural(_) => true,
            _ => false,
        }
    }

    pub fn is_blank(&self) -> bool {
        match self {
            &Message::Empty => true,
            &Message::Singular(ref s) => s == "",
            &Message::Plural(ref m) => m.values().all(|s| s == ""),
        }
    }

    pub fn singular(&self) -> Option<&str> {
        match self {
            &Message::Singular(ref s) => Some(s.as_ref()),
            _ => None,
        }
    }
}

impl Default for Message {
    fn default() -> Message { Message::Empty }
}

/// Note (comment) origins.
#[derive(Clone,Debug,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub enum Origin {
    /// Comment from developer.
    Developer,
    /// Comment from translator.
    Translator,
    /// Comment with explicit author or context tag.
    Tag(String),
}

/// Translation state.
///
/// Indicates whether the translation is considered usable.
///
/// # TODO
///
/// - Rejected, Unreviewed, NeedsReview (from TT), possibly more (note: obsolete is a separate
///   flag)
#[derive(Copy,Clone,PartialEq,Eq,PartialOrd,Ord,Debug,Hash)]
pub enum State {
    /// The unit is not translated.
    Empty,
    /// The unit is a suggestion that might be embarrassingly wrong, possibly automatic. It needs
    /// checking by human translator before it can be used. (Used for `#,fuzzy` entries in `.po`.)
    NeedsWork,
    /// The unit is considered usable.
    Final,
}

impl Default for State {
    fn default() -> State { State::Empty }
}

/// Elementary unit of translation.
///
/// A translation unit contains:
///
/// - One *source* string, the original message.
/// - At most one *target* string, the translated message.
/// - Optional *context* string that disambiguates the original.
/// - A status. This indicates whether the unit is usable in the software.
///
/// Additionally, it can also contain:
///  - Notes, from developer or translator.
///  - References back into the source where the unit is used.
///  - Previous source and context if the target is automatic suggestion from fuzzy matching.
///  - Obsolete flag, indicating the unit is not currently in use.
#[derive(Clone,Debug,Default)]
pub struct Unit {
    _context: Option<String>,
    _source: Message,
    _target: Message,
    _prev_context: Option<String>,
    _prev_source: Message,
    _notes: Vec<(Origin, String)>,
    _locations: Vec<String>,
    _state: State,
    _obsolete: bool,
}

impl Unit {
    /// Get the context string.
    pub fn context(&self) -> &Option<String> { &self._context }
    /// Get the source string.
    pub fn source(&self) -> &Message { &self._source }
    /// Get the target string.
    pub fn target(&self) -> &Message { &self._target }
    /// Get the previous context (in fuzzy units).
    pub fn prev_context(&self) -> &Option<String> { &self._prev_context }
    /// Get the previous source (in fuzzy units).
    pub fn prev_source(&self) -> &Message { &self._prev_source }
    /// Get the notes/comments.
    pub fn notes(&self) -> &Vec<(Origin, String)> { &self._notes }
    /// Get locations.
    pub fn locations(&self) -> &Vec<String> { &self._locations }
    /// Get the state.
    pub fn state(&self) -> State { self._state }
    /// Returns whether the unit should be used in application.
    pub fn is_translated(&self) -> bool { self._state == State::Final }
    /// Returns whether the unit is obsolete.
    pub fn is_obsolete(&self) -> bool { self._obsolete }
}

/// Catalogue reader.
///
/// Defines common interface of catalogue readers. Read the units by simply iterating over the
/// reader. The other methods are for the important metadata.
pub trait CatalogueReader : Iterator<Item = Result<Unit, Error>> {
    fn target_language(&self) -> &LanguageRange<'static>;
    // TODO: More attributes, possibly a generic API
}

/// Error in reading (and, in future, writing) a catalogue.
#[derive(Debug)]
pub enum Error {
    /// An I/O error from file operation.
    ///
    /// The first parameter is line number if applicable, the second is the system error.
    Io(usize, std::io::Error),
    /// A parse error.
    ///
    /// Parameters are line number, optional unexpected token and an array of expected tokens.
    /// Unset unexpected token means the parser is not smart enough to remember what it stopped on.
    /// Empty array of expected items means the parser is not smart enough to remember what it
    /// could have accepted instead.
    Parse(usize, Option<String>, Vec<&'static str>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            &Error::Io(0, ref err) => err.fmt(f),
            &Error::Io(line, ref err) => write!(f, "{} at line {}", err, line),
            &Error::Parse(line, ref got, ref exp) => {
                write!(f, "Parse error at line {}", line)?;
                if !exp.is_empty() {
                    let mut prefix = ", expected";
                    for e in exp {
                        write!(f, "{} ‘{}’", prefix, e)?;
                        prefix = " or";
                    }
                }
                if got.is_some() {
                    write!(f, ", got ‘{}’", got.as_ref().unwrap())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        self.cause().map(std::error::Error::description).unwrap_or("parse error")
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match self {
            &Error::Io(_, ref err) => Some(err),
            &Error::Parse(..) => None,
        }
    }
}

// Note: tests in each submodule
