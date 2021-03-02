//! # gettext C library FFI binding for Rust
//!
//! Usage:
//!
//! ```rust,no_run
//! use gettextrs::*;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     textdomain("hellorust")?;
//!     bindtextdomain("hellorust", "/usr/local/share/locale")?;
//!
//!     // It's sufficient to call any one of those two. See "UTF-8 is required" section below.
//!     setlocale(LocaleCategory::LcAll, "en_US.UTF-8");
//!     bind_textdomain_codeset("hellorust", "UTF-8")?;
//!
//!     println!("Translated: {}", gettext("Hello, world!"));
//!     println!("Singular: {}", ngettext("One thing", "Multiple things", 1));
//!     println!("Plural: {}", ngettext("One thing", "Multiple things", 2));
//!     println!("With placeholder: {}", gettext!("Hello, {}!",  "Example User"));
//!
//!     Ok(())
//! }
//! ```
//!
//! Alternatively, you can initialize the locale and text domain using the [`TextDomain`] builder.
//! By default, a translation of the specified text domain in current language is searched in
//! the system's data paths. See [`TextDomain`]'s documentation for other options.
//!
//! ```rust,no_run
//! use gettextrs::TextDomain;
//!
//! TextDomain::new("hellorust")
//!            .init()
//!            .unwrap();
//! ```
//!
//! [`TextDomain`]: struct.TextDomain.html
//!
//! ## UTF-8 is required
//!
//! By default, gettext converts results to the locale's codeset. Rust, on the other hand, uses
//! UTF-8 regardless of the locale. There's no universal way to bridge that gap, so this crate
//! doesn't even try. Instead, *you* have to do at least one of the following:
//!
//! 1. force gettext to encode its results into UTF-8, either by calling an appropriate function:
//!
//!     ```rust,no_run
//!     # use gettextrs::*;
//!     # fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     bind_textdomain_codeset("hellorust", "UTF-8")?;
//!     # Ok(())
//!     # }
//!     ```
//!
//!     ...or using [`TextDomain`] builder:
//!
//!     ```rust,no_run
//!     # use gettextrs::*;
//!     TextDomain::new("hellorust")
//!         .codeset("UTF-8") // Optional, the builder does this by default
//!         .init()
//!         .unwrap();
//!     ```
//!
//! 2. change into a locale that uses UTF-8, either by calling an appropriate function:
//!
//!     ```rust,no_run
//!     # use gettextrs::*;
//!     setlocale(LocaleCategory::LcAll, "en_US.UTF-8");
//!     // or just for messages:
//!     setlocale(LocaleCategory::LcMessages, "en_US.UTF-8");
//!     ```
//!
//!     ...or using [`TextDomain`] builder:
//!
//!     ```rust,no_run
//!     # use gettextrs::*;
//!     TextDomain::new("hellorust")
//!         .locale("en_US.UTF-8")
//!         .init()
//!         .unwrap();
//!     ```
//!
//! If you don't do any of that, calls to `gettext()` and other functions might panic when they
//! encounter something that isn't UTF-8. They can also garble data as they interpret the other
//! encoding as UTF-8.

extern crate locale_config;

extern crate gettext_sys as ffi;

use std::ffi::CStr;
use std::ffi::CString;
use std::io;
use std::os::raw::c_ulong;
use std::path::PathBuf;

pub mod macros;
pub use macros::*;
mod text_domain;
pub use text_domain::{TextDomain, TextDomainError};
pub mod getters;

/// Locale category enum ported from locale.h.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LocaleCategory {
    /// Character classification and case conversion.
    LcCType = 0,
    /// Non-monetary numeric formats.
    LcNumeric = 1,
    /// Date and time formats.
    LcTime = 2,
    /// Collation order.
    LcCollate = 3,
    /// Monetary formats.
    LcMonetary = 4,
    /// Formats of informative and diagnostic messages and interactive responses.
    LcMessages = 5,
    /// For all.
    LcAll = 6,
    /// Paper size.
    LcPaper = 7,
    /// Name formats.
    LcName = 8,
    /// Address formats and location information.
    LcAddress = 9,
    /// Telephone number formats.
    LcTelephone = 10,
    /// Measurement units (Metric or Other).
    LcMeasurement = 11,
    /// Metadata about the locale information.
    LcIdentification = 12,
}

/// Translate msgid to localized message from default domain.
///
/// # Panics
///
/// Panics if:
///
/// * `s` contains an internal 0 byte, as such values can't be passed to the gettext's C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn gettext<T: Into<String>>(s: T) -> String {
    let s = CString::new(s.into()).expect("`s` contains an internal 0 byte");
    unsafe {
        CStr::from_ptr(ffi::gettext(s.as_ptr()))
            .to_str()
            .expect("gettext() returned invalid UTF-8")
            .to_owned()
    }
}

/// Translate msgid to localized message from specified domain.
///
/// # Panics
///
/// Panics if:
///
/// * `domain` or `s` contain an internal 0 byte, as such values can't be passed to the gettext's
///     C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn dgettext<T, U>(domain: T, s: U) -> String
where
    T: Into<String>,
    U: Into<String>,
{
    let domain = CString::new(domain.into()).expect("`domain` contains an internal 0 byte");
    let s = CString::new(s.into()).expect("`s` contains an internal 0 byte");
    unsafe {
        CStr::from_ptr(ffi::dgettext(domain.as_ptr(), s.as_ptr()))
            .to_str()
            .expect("dgettext() returned invalid UTF-8")
            .to_owned()
    }
}

/// Translate msgid to localized message from specified domain using custom locale category.
///
/// # Panics
///
/// Panics if:
/// * `domain` or `s` contain an internal 0 byte, as such values can't be passed to the gettext's
///     C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn dcgettext<T, U>(domain: T, s: U, category: LocaleCategory) -> String
where
    T: Into<String>,
    U: Into<String>,
{
    let domain = CString::new(domain.into()).expect("`domain` contains an internal 0 byte");
    let s = CString::new(s.into()).expect("`s` contains an internal 0 byte");
    unsafe {
        CStr::from_ptr(ffi::dcgettext(domain.as_ptr(), s.as_ptr(), category as i32))
            .to_str()
            .expect("dcgettext() returned invalid UTF-8")
            .to_owned()
    }
}

/// Translate msgid to localized message from default domain (with plural support).
///
/// # Panics
///
/// Panics if:
/// * `singular` or `plural` contain an internal 0 byte, as such values can't be passed to the
///     gettext's C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn ngettext<T, S>(singular: T, plural : S, n : u32) -> String
where
    T: Into<String>,
    S: Into<String>,
{
    let singular = CString::new(singular.into()).expect("`singular` contains an internal 0 byte");
    let plural = CString::new(plural.into()).expect("`plural` contains an internal 0 byte");
    unsafe {
        CStr::from_ptr(ffi::ngettext(singular.as_ptr(), plural.as_ptr(), n as c_ulong))
            .to_str()
            .expect("ngettext() returned invalid UTF-8")
            .to_owned()
    }
}

/// Translate msgid to localized message from specified domain (with plural support).
///
/// # Panics
///
/// Panics if:
/// * `domain`, `singular`, or `plural` contain an internal 0 byte, as such values can't be passed
///     to the gettext's C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn dngettext<T, U, V>(domain: T, singular: U, plural: V, n : u32) -> String
where
    T: Into<String>,
    U: Into<String>,
    V: Into<String>,
{
    let domain = CString::new(domain.into()).expect("`domain` contains an internal 0 byte");
    let singular = CString::new(singular.into()).expect("`singular` contains an internal 0 byte");
    let plural = CString::new(plural.into()).expect("`plural` contains an internal 0 byte");
    unsafe {
        CStr::from_ptr(ffi::dngettext(domain.as_ptr(), singular.as_ptr(), plural.as_ptr(), n as c_ulong))
            .to_str()
            .expect("dngettext() returned invalid UTF-8")
            .to_owned()
    }
}

/// Translate msgid to localized message from specified domain using custom locale category (with plural support).
///
/// # Panics
///
/// Panics if:
/// * `domain`, `singular`, or `plural` contain an internal 0 byte, as such values can't be passed
///     to the gettext's C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn dcngettext<T, U, V>(domain: T, singular: U, plural: V, n : u32, category: LocaleCategory) -> String
where
    T: Into<String>,
    U: Into<String>,
    V: Into<String>,
{
    let domain = CString::new(domain.into()).expect("`domain` contains an internal 0 byte");
    let singular = CString::new(singular.into()).expect("`singular` contains an internal 0 byte");
    let plural = CString::new(plural.into()).expect("`plural` contains an internal 0 byte");
    unsafe {
        CStr::from_ptr(ffi::dcngettext(domain.as_ptr(), singular.as_ptr(), plural.as_ptr(), n as c_ulong, category as i32))
            .to_str()
            .expect("dcngettext() returned invalid UTF-8")
            .to_owned()
    }
}

/// Switch to specific text domain.
///
/// Returns the current domain, after possibly changing it. (There's no trailing 0 byte in the
/// return value.)
///
/// If you want to *get* current domain, rather than set it, use [`getters::current_textdomain`].
///
/// # Panics
///
/// Panics if `domain` contains an internal 0 byte, as such values can't be passed to the gettext's
/// C API.
pub fn textdomain<T: Into<Vec<u8>>>(domain: T) -> Result<Vec<u8>, io::Error> {
    let domain = CString::new(domain).expect("`domain` contains an internal 0 byte");
    unsafe {
        let result = ffi::textdomain(domain.as_ptr());
        if result.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(CStr::from_ptr(result).to_bytes().to_owned())
        }
    }
}

/// Bind text domain to some directory containing gettext MO files.
///
/// Returns the current directory for given domain, after possibly changing it.
///
/// If you want to *get* domain directory, rather than set it, use [`getters::domain_directory`].
///
/// # Panics
///
/// Panics if `domain` or `dir` contain an internal 0 byte, as such values can't be passed to the
/// gettext's C API.
pub fn bindtextdomain<T, U>(domain: T, dir: U) -> Result<PathBuf, io::Error>
where
    T: Into<Vec<u8>>,
    U: Into<PathBuf>,
{
    let domain = CString::new(domain).expect("`domain` contains an internal 0 byte");
    let dir = dir.into().into_os_string();

    #[cfg(windows)]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::{OsStrExt, OsStringExt};

        let mut dir: Vec<u16> = dir.encode_wide().collect();
        if dir.contains(&0) {
            panic!("`dir` contains an internal 0 byte");
        }
        // Trailing zero to mark the end of the C string.
        dir.push(0);
        unsafe {
            let mut ptr = ffi::wbindtextdomain(domain.as_ptr(), dir.as_ptr());
            if ptr.is_null() {
                Err(io::Error::last_os_error())
            } else {
                let mut result = vec![];
                while *ptr != 0_u16 {
                    result.push(*ptr);
                    ptr = ptr.offset(1);
                }
                Ok(PathBuf::from(OsString::from_wide(&result)))
            }
        }
    }

    #[cfg(not(windows))]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let dir = dir.into_vec();
        let dir = CString::new(dir).expect("`dir` contains an internal 0 byte");
        unsafe {
            let result = ffi::bindtextdomain(domain.as_ptr(), dir.as_ptr());
            if result.is_null() {
                Err(io::Error::last_os_error())
            } else {
                let result = CStr::from_ptr(result);
                Ok(PathBuf::from(OsString::from_vec(result.to_bytes().to_vec())))
            }
        }
    }
}

/// Set current locale for translations.
///
/// Returns an opaque string that describes the locale set. You can pass that string into
/// `setlocale()` later to set the same local again. `None` means the call failed (the underlying
/// API doesn't provide any details).
///
/// # Panics
///
/// Panics if `locale` contains an internal 0 byte, as such values can't be passed to the gettext's
/// C API.
pub fn setlocale<T: Into<Vec<u8>>>(category: LocaleCategory, locale: T) -> Option<Vec<u8>> {
    let c = CString::new(locale).expect("`locale` contains an internal 0 byte");
    unsafe {
        let ret = ffi::setlocale(category as i32, c.as_ptr());
        if ret.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ret).to_bytes().to_owned())
        }
    }
}

/// Set encoding of translated messages.
///
/// Returns the current charset for given domain, after possibly changing it. `None` means no
/// codeset has been set.
///
/// If you want to *get* current encoding, rather than set it, use [`getters::textdomain_codeset`].
///
/// # Panics
///
/// Panics if:
/// * `domain` or `codeset` contain an internal 0 byte, as such values can't be passed to the
///     gettext's C API;
/// * the result is not in UTF-8 (which shouldn't happen as the results should always be ASCII, as
///     they're just codeset names).
pub fn bind_textdomain_codeset<T, U>(domain: T, codeset: U) -> Result<Option<String>, io::Error>
where
    T: Into<Vec<u8>>,
    U: Into<String>,
{
    let domain = CString::new(domain).expect("`domain` contains an internal 0 byte");
    let codeset = CString::new(codeset.into()).expect("`codeset` contains an internal 0 byte");
    unsafe {
        let result = ffi::bind_textdomain_codeset(domain.as_ptr(), codeset.as_ptr());
        if result.is_null() {
            let error = io::Error::last_os_error();
            if let Some(0) = error.raw_os_error() {
                return Ok(None)
            } else {
                return Err(error)
            }
        } else {
            let result =
                CStr::from_ptr(result)
                .to_str()
                .expect("`bind_textdomain_codeset()` returned non-UTF-8 string")
                .to_owned();
            Ok(Some(result))
        }
    }
}

static CONTEXT_SEPARATOR: char = '\x04';

fn build_context_id(ctx: &str, s: &str) -> String {
    format!("{}{}{}", ctx, CONTEXT_SEPARATOR, s)
}

fn panic_on_zero_in_ctx(string: &str) {
    if string.contains('\0') {
        panic!("`ctx` contains an internal 0 byte");
    }
}

/// Translate msgid to localized message from default domain (with context support).
///
/// # Panics
///
/// Panics if:
/// * `ctx` or `s` contain an internal 0 byte, as such values can't be passed to the gettext's
///     C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn pgettext<T, U>(ctx: T, s: U) -> String
where
    T: Into<String>,
    U: Into<String>,
{
    let ctx = ctx.into();
    panic_on_zero_in_ctx(&ctx);

    let msgid = s.into();
    let text = build_context_id(&ctx, &msgid);

    let trans = gettext(text);
    if trans.contains(CONTEXT_SEPARATOR as char) {
        return gettext(msgid);
    }

    trans
}

/// Translate msgid to localized message from default domain (with plural support and context
/// support).
///
/// # Panics
///
/// Panics if:
/// * `ctx`, `singular`, or `plural` contain an internal 0 byte, as such values can't be passed to
///     the gettext's C API;
/// * the result is not in UTF-8 (see [this note](./index.html#utf-8-is-required)).
pub fn npgettext<T, U, V>(ctx: T, singular: U, plural: V, n: u32) -> String
where
    T: Into<String>,
    U: Into<String>,
    V: Into<String>,
{
    let ctx = ctx.into();
    panic_on_zero_in_ctx(&ctx);

    let singular_msgid = singular.into();
    let plural_msgid = plural.into();
    let singular_ctx = build_context_id(&ctx, &singular_msgid);
    let plural_ctx = build_context_id(&ctx, &plural_msgid);

    let trans = ngettext(singular_ctx, plural_ctx, n);
    if trans.contains(CONTEXT_SEPARATOR as char) {
        return ngettext(singular_msgid, plural_msgid, n);
    }

    trans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        setlocale(LocaleCategory::LcAll, "en_US.UTF-8");

        bindtextdomain("hellorust", "/usr/local/share/locale").unwrap();
        textdomain("hellorust").unwrap();

        assert_eq!("Hello, world!", gettext("Hello, world!"));
    }

    #[test]
    fn plural_test() {
        setlocale(LocaleCategory::LcAll, "en_US.UTF-8");

        bindtextdomain("hellorust", "/usr/local/share/locale").unwrap();
        textdomain("hellorust").unwrap();

        assert_eq!("Hello, world!", ngettext("Hello, world!", "Hello, worlds!", 1));
        assert_eq!("Hello, worlds!", ngettext("Hello, world!", "Hello, worlds!", 2));
    }

    #[test]
    fn context_test() {
        setlocale(LocaleCategory::LcAll, "en_US.UTF-8");

        bindtextdomain("hellorust", "/usr/local/share/locale").unwrap();
        textdomain("hellorust").unwrap();

        assert_eq!("Hello, world!", pgettext("context", "Hello, world!"));
    }

    #[test]
    fn plural_context_test() {
        setlocale(LocaleCategory::LcAll, "en_US.UTF-8");

        bindtextdomain("hellorust", "/usr/local/share/locale").unwrap();
        textdomain("hellorust").unwrap();

        assert_eq!("Hello, world!", npgettext("context", "Hello, world!", "Hello, worlds!", 1));
        assert_eq!("Hello, worlds!", npgettext("context", "Hello, world!", "Hello, worlds!", 2));
    }

    #[test]
    #[should_panic(expected = "`s` contains an internal 0 byte")]
    fn gettext_panics() {
        gettext("input string\0");
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn dgettext_panics_on_zero_in_domain() {
        dgettext("hello\0world!", "hi");
    }

    #[test]
    #[should_panic(expected = "`s` contains an internal 0 byte")]
    fn dgettext_panics_on_zero_in_s() {
        dgettext("hello world", "another che\0ck");
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn dcgettext_panics_on_zero_in_domain() {
        dcgettext("a diff\0erent input", "hello", LocaleCategory::LcAll);
    }

    #[test]
    #[should_panic(expected = "`s` contains an internal 0 byte")]
    fn dcgettext_panics_on_zero_in_s() {
        dcgettext("world", "yet \0 another\0 one", LocaleCategory::LcMessages);
    }

    #[test]
    #[should_panic(expected = "`singular` contains an internal 0 byte")]
    fn ngettext_panics_on_zero_in_singular() {
        ngettext("singular\0form", "plural form", 10);
    }

    #[test]
    #[should_panic(expected = "`plural` contains an internal 0 byte")]
    fn ngettext_panics_on_zero_in_plural() {
        ngettext("singular form", "plural\0form", 0);
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn dngettext_panics_on_zero_in_domain() {
        dngettext("do\0main", "one", "many", 0);
    }

    #[test]
    #[should_panic(expected = "`singular` contains an internal 0 byte")]
    fn dngettext_panics_on_zero_in_singular() {
        dngettext("domain", "just a\0 single one", "many", 100);
    }

    #[test]
    #[should_panic(expected = "`plural` contains an internal 0 byte")]
    fn dngettext_panics_on_zero_in_plural() {
        dngettext("d", "1", "many\0many\0many more", 10000);
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn dcngettext_panics_on_zero_in_domain() {
        dcngettext("doma\0in", "singular", "plural", 42, LocaleCategory::LcCType);
    }

    #[test]
    #[should_panic(expected = "`singular` contains an internal 0 byte")]
    fn dcngettext_panics_on_zero_in_singular() {
        dcngettext("domain", "\0ne", "plural", 13, LocaleCategory::LcNumeric);
    }

    #[test]
    #[should_panic(expected = "`plural` contains an internal 0 byte")]
    fn dcngettext_panics_on_zero_in_plural() {
        dcngettext("d-o-m-a-i-n", "one", "a\0few", 0, LocaleCategory::LcTime);
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn textdomain_panics_on_zero_in_domain() {
        textdomain("this is \0 my domain").unwrap();
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn bindtextdomain_panics_on_zero_in_domain() {
        bindtextdomain("\0bind this", "/usr/share/locale").unwrap();
    }

    #[test]
    #[should_panic(expected = "`dir` contains an internal 0 byte")]
    fn bindtextdomain_panics_on_zero_in_dir() {
        bindtextdomain("my_domain", "/opt/locales\0").unwrap();
    }

    #[test]
    #[should_panic(expected = "`locale` contains an internal 0 byte")]
    fn setlocale_panics_on_zero_in_locale() {
        setlocale(LocaleCategory::LcCollate, "en_\0US");
    }

    #[test]
    #[should_panic(expected = "`domain` contains an internal 0 byte")]
    fn bind_textdomain_codeset_panics_on_zero_in_domain() {
        bind_textdomain_codeset("doma\0in", "UTF-8").unwrap();
    }

    #[test]
    #[should_panic(expected = "`codeset` contains an internal 0 byte")]
    fn bind_textdomain_codeset_panics_on_zero_in_codeset() {
        bind_textdomain_codeset("name", "K\0I8-R").unwrap();
    }

    #[test]
    #[should_panic(expected = "`ctx` contains an internal 0 byte")]
    fn pgettext_panics_on_zero_in_ctx() {
        pgettext("context\0", "string");
    }

    #[test]
    #[should_panic(expected = "`s` contains an internal 0 byte")]
    fn pgettext_panics_on_zero_in_s() {
        pgettext("ctx", "a message\0to be translated");
    }

    #[test]
    #[should_panic(expected = "`ctx` contains an internal 0 byte")]
    fn npgettext_panics_on_zero_in_ctx() {
        npgettext("c\0tx", "singular", "plural", 0);
    }

    #[test]
    #[should_panic(expected = "`singular` contains an internal 0 byte")]
    fn npgettext_panics_on_zero_in_singular() {
        npgettext("ctx", "sing\0ular", "many many more", 135626);
    }

    #[test]
    #[should_panic(expected = "`plural` contains an internal 0 byte")]
    fn npgettext_panics_on_zero_in_plural() {
        npgettext("context", "uno", "one \0fewer", 10585);
    }
}