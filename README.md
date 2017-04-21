[![TravisCI Build Status](https://travis-ci.org/rust-locale/translate-storage.svg?branch=master)](https://travis-ci.org/rust-locale/translate-storage)
[![AppVeyor Build Status](https://ci.appveyor.com/api/projects/status/jahhyc6w17kk2wbj/branch/master?svg=true)](https://ci.appveyor.com/project/jan-hudec/translate-storage/branch/master)
[![Crates.io Version](https://img.shields.io/crates/v/translate-storage.svg)](https://crates.io/crates/translate-storage)
[![Docs.rs](https://docs.rs/translate-storage/badge.svg)](https://docs.rs/translate-storage/)

# `translate-storage`

Rust library for reading, and in future writing, translation catalogs in
Uniforum/Gettext PO and (in future) Xliff formats. Similar to the
[translate.storage] package in Python [Translate Toolkit].

Only PO and Xliff are planned to be supported. For anything else, just convert
it with [Translate Toolkit]. There is no point in replacing that excellent
library; the main reason for Rust parser and writer is to them as part of build
process of Rust programs, especially in procedural macros, which need to be
written in Rust.

## Documentation

On [![Docs.rs](https://docs.rs/translate-storage/badge.svg)](https://docs.rs/locale/).

## Installation

It uses [Cargo](http://crates.io), Rust's package manager. You can depend on this library by adding `translate-storage` to your Cargo dependencies:

```toml
[dependencies]
translate-storage = "0.1"
```

Or, to use the Git repo directly:
```toml
[dependencies.translate-storage]
git = "https://github.com/rust-locale/translate-storagee.git"
```


[translate.storage]: http://docs.translatehouse.org/projects/translate-toolkit/en/latest/api/storage.html
[Translate Toolkit]: http://docs.translatehouse.org/projects/translate-toolkit/
