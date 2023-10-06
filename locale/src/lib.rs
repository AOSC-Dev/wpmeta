mod de;
mod error;

use isolang::Language;
use serde::Deserialize;

use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;
use std::sync::OnceLock;

pub use error::LocaleError;

#[derive(Clone, Debug, Deserialize)]
pub struct Locale {
    locale: String,
    #[serde(skip)]
    language: OnceLock<Option<Language>>,
}

#[derive(Clone, Debug)]
pub struct Localized<T> {
    default: Option<T>,
    content: HashMap<Locale, T>,
}

impl PartialEq for Locale {
    fn eq(&self, other: &Self) -> bool {
        self.locale.eq(&other.locale)
    }
}

impl Eq for Locale {}

impl Hash for Locale {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.locale.hash(state)
    }
}

impl Locale {
    pub fn new<S: AsRef<str>>(locale: S) -> Self {
        Self {
            locale: locale.as_ref().into(),
            language: OnceLock::new(),
        }
    }

    pub fn to_locale(&self) -> &str {
        &self.locale
    }

    fn get_language(&self) -> Option<&Language> {
        self.language
            .get_or_init(|| Language::from_locale(&self.locale))
            .as_ref()
    }

    pub fn to_iso639_1(&self) -> Option<&str> {
        self.get_language().map(|l| l.to_639_1()).flatten()
    }

    pub fn to_iso639_3(&self) -> Option<&str> {
        self.get_language().map(|l| l.to_639_3())
    }
}

impl FromStr for Locale {
    type Err = LocaleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            locale: s.into(),
            language: OnceLock::new(),
        })
    }
}

impl<T> Localized<T> {
    pub fn new(default: Option<T>) -> Self {
        Self {
            default,
            content: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.content.len() + self.default.as_ref().map(|_| 1).unwrap_or(0)
    }

    pub fn set(&mut self, locale: Locale, content: T) -> Option<T> {
        self.content.insert(locale, content)
    }

    pub fn generate_hashmap<F>(&self, transform: F) -> Result<HashMap<&str, &T>, LocaleError>
    where
        F: Fn(&Locale) -> &str,
    {
        Ok(self
            .content
            .iter()
            .map(|(locale, value)| (transform(locale), value))
            .collect())
    }

    pub fn get_default(&self) -> Option<&T> {
        self.default.as_ref()
    }
}

impl<T: PartialEq> PartialEq for Localized<T> {
    fn eq(&self, other: &Self) -> bool {
        self.default.eq(&other.default) && self.content.eq(&other.content)
    }
}

impl<T: Eq> Eq for Localized<T> {}
