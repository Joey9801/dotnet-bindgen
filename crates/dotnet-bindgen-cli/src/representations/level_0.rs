//! This module defines the final structured representation before a source code string

use core::iter::FromIterator;
use std::io;

pub type LayerEntrypoint = TokenStream;

#[derive(Debug, Clone, Copy)]
pub enum Delimiter {
    /// { ... }
    Brace,

    /// ( ... )
    Paren,

    /// [ ... ]
    Bracket,

    /// ...
    None,
}

impl Delimiter {
    fn open(&self) -> char {
        match self {
            Delimiter::Brace => '{',
            Delimiter::Paren => '(',
            Delimiter::Bracket => '[',
            Delimiter::None => ' ',
        }
    }

    fn close(&self) -> char {
        match self {
            Delimiter::Brace => '}',
            Delimiter::Paren => ')',
            Delimiter::Bracket => ']',
            Delimiter::None => ' ',
        }
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    pub delimiter: Delimiter,
    pub content: TokenStream,
}

impl Group {
    fn render(&self, writer: &mut dyn io::Write) -> Result<(), io::Error> {
        write!(writer, "{} ", self.delimiter.open())?;
        self.content.render(writer)?;
        write!(writer, " {}", self.delimiter.close())
    }
}

#[derive(Debug, Clone)]
pub struct Ident {
    name: String,
}

impl Ident {
    pub fn new<S: ToString>(s: S) -> Self {
        Ident {
            name: s.to_string(),
        }
    }
}

impl<T: AsRef<str>> From<T> for Ident {
    fn from(s: T) -> Self {
        Self {
            name: s.as_ref().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Punct {
    Semicolon,
    Ampersand,
    Asterisk,
    Equals,
    Period,
    Comma,
    QuestionMark,
    Colon,
}

impl Punct {
    fn as_char(&self) -> char {
        match self {
            Self::Semicolon => ';',
            Self::Ampersand => '&',
            Self::Asterisk => '*',
            Self::Equals => '=',
            Self::Period => '.',
            Self::Comma => ',',
            Self::QuestionMark => '?',
            Self::Colon => ':',
        }
    }
}

#[derive(Debug, Clone)]
pub enum Formatting {
    Newline,
}

impl Formatting {
    pub fn render(&self, writer: &mut dyn io::Write) -> Result<(), io::Error> {
        match self {
            Formatting::Newline => write!(writer, "\n"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TokenTree {
    Group(Group),
    Ident(Ident),
    Punct(Punct),
    Formatting(Formatting),
}

impl TokenTree {
    pub fn render(&self, writer: &mut dyn io::Write) -> Result<(), io::Error> {
        match self {
            TokenTree::Group(g) => g.render(writer),
            TokenTree::Ident(i) => write!(writer, "{}", i.name),
            TokenTree::Punct(p) => write!(writer, "{}", p.as_char()),
            TokenTree::Formatting(f) => f.render(writer),
        }
    }
}

impl From<Group> for TokenTree {
    fn from(g: Group) -> Self {
        Self::Group(g)
    }
}

impl<T: Into<Ident>> From<T> for TokenTree {
    fn from(i: T) -> Self {
        Self::Ident(i.into())
    }
}

impl From<Punct> for TokenTree {
    fn from(p: Punct) -> Self {
        Self::Punct(p)
    }
}

impl From<Formatting> for TokenTree {
    fn from(f: Formatting) -> Self {
        Self::Formatting(f)
    }
}

#[derive(Debug, Clone)]
pub struct TokenStream {
    parts: Vec<TokenTree>,
}

impl TokenStream {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a TokenTree> {
        self.parts.iter()
    }

    pub fn push<T: Into<TokenTree>>(&mut self, elem: T) {
        self.parts.push(elem.into());
    }

    pub fn render(&self, writer: &mut dyn io::Write) -> Result<(), io::Error> {
        let mut first = true;
        for part in &self.parts {
            if !first {
                write!(writer, " ")?;
            }
            first = false;

            part.render(writer)?;
        }
        Ok(())
    }
}

impl<T: Into<TokenTree>> Extend<T> for TokenStream {
    fn extend<I: IntoIterator<Item = T>>(&mut self, i: I) {
        for elem in i {
            self.parts.push(elem.into());
        }
    }
}

impl<T: Into<TokenTree>> FromIterator<T> for TokenStream {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let parts = iter.into_iter().map(|x| x.into()).collect();
        Self { parts }
    }
}

pub trait ToTokens {
    fn to_tokens(&self, tokens: &mut TokenStream);

    fn to_token_stream(&self) -> TokenStream {
        let mut tokens = TokenStream::new();
        self.to_tokens(&mut tokens);
        tokens
    }
}
