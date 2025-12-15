//! # Localized
//!
//! Localized container based on `BTreeMap`, with serde support.
//!
//! ```rust
//! use localized::{Locale, Localized};
//!
//! let mut localized = Localized::new(Some("Default string goes here"));
//! // Get default value with `.get_default()`
//! assert_eq!(localized.get_default(), Some("Default string goes here").as_ref());
//!
//! // Insert string with locale
//! localized.insert(Locale::new("en-US"), "String for en_US");
//! assert_eq!(localized["en_US"], "String for en_US");
//! assert_eq!(localized["en-US"], "String for en_US");  // `-` and `_` both works
//!
//! // Fallbacks to the default value with unknown locales
//! assert_eq!(localized["zh_CN"], "Default string goes here");
//!
//! ```

mod de;
pub mod error;
mod ser;

use std::collections::BTreeMap;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Index;
use std::str::FromStr;

pub use error::LocaleError;

/// Simple representation of a locale
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Locale {
    lang: String,
    region: Option<String>,
}

/// Container for localized data
#[derive(Clone, Debug)]
pub struct Localized<T> {
    pub default: Option<T>,
    pub content: BTreeMap<Locale, T>,
}

impl Locale {
    /// Created a new instance of `Locale`
    pub fn new<S: AsRef<str>>(locale: S) -> Self {
        let locale_str = locale.as_ref().replace('-', "_");
        let (lang, region) = match locale_str.split_once('_') {
            Some((l, r)) => (l.to_lowercase(), Some(r.to_uppercase())),
            None => (locale_str, None),
        };
        Self { lang, region }
    }

    /// Get the language part of the `Locale`
    pub fn get_lang(&self) -> &str {
        &self.lang
    }

    /// Get the region part of the `Locale`
    pub fn get_region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Get concatenated locale name
    pub fn get_locale<S>(&self, delimiter: S) -> String
    where
        S: AsRef<str>
    {
        match &self.region {
            None => self.lang.to_owned(),
            Some(region) => format!("{}{}{}", self.lang, delimiter.as_ref(), region),
        }
    }
}

impl FromStr for Locale {
    type Err = LocaleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_locale("_"))
    }
}

impl Hash for Locale {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.lang.hash(state);
        self.region.hash(state);
    }
}

impl<T> Localized<T> {
    /// Create a new instance of `Localized`
    pub fn new(default: Option<T>) -> Self {
        Self {
            default,
            content: BTreeMap::new(),
        }
    }

    /// Length of the container
    pub fn len(&self) -> usize {
        self.content.len() + self.default.as_ref().map(|_| 1).unwrap_or(0)
    }

    /// Check if the container is empty or not
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Set the data for a specific locale
    pub fn insert(&mut self, locale: Locale, content: T) -> Option<T> {
        self.content.insert(locale, content)
    }

    /// Update the default value
    pub fn default(&mut self, default: Option<T>) {
        self.default = default;
    }

    /// Convert to a `HashMap`[std::collections::HashMap]
    pub fn to_hashmap<F>(&self, transform: F) -> Result<BTreeMap<String, &T>, LocaleError>
    where
        F: Fn(&Locale) -> String,
    {
        Ok(self
            .content
            .iter()
            .map(|(locale, value)| (transform(locale), value))
            .collect())
    }

    /// Get the default value
    pub fn get_default(&self) -> Option<&T> {
        self.default.as_ref()
    }
}

impl<T> Index<&Locale> for Localized<T> {
    type Output = T;

    fn index(&self, index: &Locale) -> &Self::Output {
        if self.content.contains_key(index) {
            self.content.index(index)
        } else {
            self.get_default()
                .expect("Key not found and no default value specified")
        }
    }
}

impl<T, S> Index<S> for Localized<T>
where
    S: AsRef<str>,
{
    type Output = T;

    fn index(&self, index: S) -> &Self::Output {
        let locale = Locale::new(index.as_ref());
        self.index(&locale)
    }
}

impl<T: PartialEq> PartialEq for Localized<T> {
    fn eq(&self, other: &Self) -> bool {
        self.default.eq(&other.default) && self.content.eq(&other.content)
    }
}

impl<T: Eq> Eq for Localized<T> {}

#[cfg(test)]
mod test {
    use super::{Locale, Localized};
    use serde_test::{assert_tokens, Token};
    use std::collections::BTreeMap;

    #[test]
    fn test_access() {
        let localized = Localized::<String> {
            default: Some("Default".into()),
            content: BTreeMap::from([
                (Locale::new("en_US"), "Turtle".into()),
                (Locale::new("zh_CN"), "乌龟".into()),
                (Locale::new("zh_TW"), "烏龜".into()),
            ]),
        };
        assert_eq!(localized["zh_CN"], "乌龟");
        assert_eq!(localized["zh-CN"], "乌龟");
        assert_eq!(localized["en-US"], "Turtle");
        assert_eq!(localized["j-J"], "Default");
    }

    #[test]
    fn test_get_locale() {
        let locale = Locale::new("en-US");
        assert_eq!(locale.to_string(), "en_US");
    }

    #[test]
    fn test_serde() {
        let orig = Localized::<String> {
            default: Some("Grass".into()),
            content: BTreeMap::from([
                (Locale::new("zh-CN"), "草".into()),
                (Locale::new("ja_CN"), "Kusa".into()),
            ]),
        };
        assert_tokens(
            &orig,
            &[
                Token::Map { len: Some(3) },
                Token::Str("default"),
                Token::Str("Grass"),
                Token::Str("ja_CN"),
                Token::Str("Kusa"),
                Token::Str("zh_CN"),
                Token::Str("草"),
                Token::MapEnd,
            ],
        );
    }
}
