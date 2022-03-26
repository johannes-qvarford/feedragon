use crate::parsing::*;
use xmltree::Element;
use xmltree::XMLNode;
use xmltree::ElementPredicate;
use chrono::prelude::*;
use std::ops::Deref;
use std::convert::TryInto;
use url::Url;
use std::borrow::Borrow;

struct AtomParser;

struct ValueContext<T>(T, String);

struct ErrorContext<'a, T>(&'a T, String);

type ElementContext<'a> = ErrorContext<'a, Element>;

impl <'a> Deref for ElementContext<'a> {
    type Target = &'a Element;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

type ElementResult<'a> = Result<ElementContext<'a>, ParsingError>;

impl <'a, T> From<&'a T> for ErrorContext<'a, T>
{
    fn from(e: &'a T) -> Self {
        ErrorContext(e, "".into())
    }
}

impl <T> ErrorContext<'_, T> {
    fn with_value<'a, U>(self, value: &'a U) -> ErrorContext<'a, U> {
        ErrorContext(value, self.1)
    }

    fn with_more_context(self, context: &str) -> Self {
        ErrorContext(self.0, format!("{}\n{}", context, self.1))
    }
}

impl <T> ValueContext<T> {
    fn with_value<U>(self, value: U) -> ValueContext<U> {
        ValueContext(value, self.1)
    }

    fn with_more_context(self, context: &str) -> Self {
        ValueContext(self.0, format!("{}\n{}", context, self.1))
    }
}

impl ElementContext<'_> {
    fn element(&self, id: &str) -> ElementResult {
         self.get_child((id, "http://www.w3.org/2005/Atom"))
            .map(|c| ErrorContext(c, self.1.clone()).with_more_context(&format!("In element '{}'", id)))
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element '{}'. Context:\n{}", id, self.1)))
    }

    fn elements(&self, id: &str) -> ValueContext<Vec<&Element>> {
        let items = self.children
            .iter()
            .filter_map(|e| match e {
                XMLNode::Element(elem) => Some(elem),
                _ => None,
            })
            .filter(|e| id.match_element(e))
            .collect();
        ValueContext(items, self.1.clone()).with_more_context(&format!("In elements '{}'", id))
    }

    fn text(&self) -> Result<ValueContext<std::borrow::Cow<str>>, ParsingError> {
        let text = self.get_text()
            .map(|e| ValueContext(e, self.1.clone()).with_more_context("In text".into()))
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing text. Context:\n{}", self.1)))?;
        Ok(text)
    }

    fn attribute(&self, name: &str) -> Result<ErrorContext<String>, ParsingError> {
        self.attributes.get(name)
            .map(|s| ErrorContext(s, self.1.clone()).with_more_context(&format!("In attribute {}", s)))
            .ok_or_else(||
                ParsingError::InvalidXmlStructure(format!(
                    "Missing attribute '{}'. Context:\n{}",
                    name, self.1)))
    }
}

impl TryInto<Url> for ErrorContext<'_, String> {

    type Error= ParsingError;

    fn try_into(self) -> Result<Url, Self::Error> {
        Url::parse(&self.0).map_err(|err| ParsingError::InvalidXmlStructure(format!("Invalid url: {}. Context:\n{}", err, self.1)))
    }
}

impl ValueContext<Vec<&Element>> {

    fn find_with_attribute_value(&self, name: &str, value: &str) -> ValueContext<Option<&Element>> {
        fn has_attribute_with_value(element: &Element, name: &str, value: &str) -> bool {
            element.attributes.get(name).map_or(false, |r| r == value)
        }

        let item = self.0
                .iter()
                .find(|e| has_attribute_with_value(e, name, value))
                .map(|e| *e);

        ValueContext(item, self.1.clone()).with_more_context(&format!("With attribute {}={}", name, value))
    }
}

impl ValueContext<Option<&Element>> {
    fn into_parsing_result(&self) -> Result<ErrorContext<Element>, ParsingError> {
        self.0.ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element. Context:\n{}", self.1)))
            .map(|e| ErrorContext(e, self.1.clone()))
    }
}

impl Parser for AtomParser {
    fn parse_feed(&self, tree: Element) -> Result<Feed, ParsingError> {
        let tree = ElementContext::from(&tree);
        Ok(Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: tree.element("title")?.text()?.0.to_string(),
            link: tree.elements("link")
                .find_with_attribute_value("rel", "self")
                .into_parsing_result()?
                .attribute("href")?
                .try_into()?,
            title: tree.element("title")?.text()?.0.to_string()
        })
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    #[test]
    fn feed_with_no_entries_can_be_parsed() {
        let feed_str = std::fs::read_to_string("src/example_empty_atom_feed.xml")
            .expect("Expected example file to exist.");
        let feed_element = Element::parse(feed_str.as_bytes()).unwrap();
        let parser = AtomParser{};
        
        let feed = parser.parse_feed(feed_element);

        let expected = Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: "Example feed".into(),
            link: "https://invidious.privacy.qvarford.net/feed/private?token=something".try_into().unwrap(),
            title: "Example feed".into()
        };
        assert_eq!(Ok(expected), feed);
    }
}

impl AtomParser {
    fn parse_entry(&self, atom_entry: Element) -> Result<Entry, ParsingError> {
        let atom_entry: ElementContext = (&atom_entry).into();
        let extract_text = |id: &str| -> Result<String, ParsingError> {
             Ok(atom_entry.element(id)?.text()?.0.to_string())
        };

        let extract_attribute = |id: &str, attribute_name: &str| -> Result<String, ParsingError> {
            Ok(atom_entry.element(id)?.attribute(attribute_name)?.0.to_string())
        };

        let extract_date_time = |id: &str| -> Result<DateTime<Utc>, ParsingError> {
            let element = atom_entry.element(id)?;
            let text = element.text()?;
            let date_time_with_offset = DateTime::parse_from_rfc3339(text.0.borrow())
                .map_err(|_dt_err| ParsingError::InvalidXmlStructure(
                    format!("Invalid rfc 3339 date time '{}' Context:\n{}", _dt_err, text.1)))?;
            return Ok(DateTime::from(date_time_with_offset))
        };

        Ok(Entry {
            title: extract_text("title")?,
            id: extract_text("id")?,
            link: extract_attribute("link", "href")?,
            summary: extract_text("title")?,
            updated: extract_date_time("updated")?
        })
    }
}

#[cfg(test)]
mod entry_tests {
    use super::*;

    #[test]
    fn invidious_entry_can_be_parsed() {
        let entry_str = std::fs::read_to_string("src/example_atom_entry.xml")
            .expect("Expected example file to exist.");
        let atom_entry = Element::parse(entry_str.as_bytes()).unwrap();
        let parser = AtomParser{};

        let entry = parser.parse_entry(atom_entry)
            .expect("Expected entry to be valid.");

        let expected = Entry {
            title: String::from("SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨"),
            id: String::from("yt:video:be8ZARHsjmc"),
            link: "http://invidious.privacy.qvarford.net/watch?v=be8ZARHsjmc".parse().unwrap(),
            summary: String::from("SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨"),
            updated: DateTime::parse_from_rfc3339("2022-03-22T07:26:01+00:00").unwrap().into(),
        };
        assert_eq!(expected, entry);
    }

    #[test]
    fn invalid_entry_gives_detailed_error() {
        let entry_str = std::fs::read_to_string("src/example_atom_entry.xml")
            .expect("Expected example file to exist.");
        let mut atom_entry = Element::parse(entry_str.as_bytes()).unwrap();
        atom_entry.get_mut_child("link").unwrap().attributes.remove("href");
        let parser = AtomParser{};

        let entry = parser.parse_entry(atom_entry.clone());

        assert_eq!(Err(ParsingError::InvalidXmlStructure(
                format!("Missing attribute 'href'. Context:\nIn element 'link'\n"))),
            entry)
    }
}