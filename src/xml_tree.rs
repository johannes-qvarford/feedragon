use crate::parsing::*;
use xmltree::Element;
use xmltree::XMLNode;
use xmltree::ElementPredicate;
use std::convert::TryInto;
use url::Url;

pub struct Context<T>(T, String);

pub type ElementContext<'a> = Context<&'a Element>;

type ParsingResult<T> = Result<T, ParsingError>;

type ElementResult<'a> = ParsingResult<ElementContext<'a>>;

// TODO:
// Consider changing Context to a struct.
// Kee it DRY for manual "Context: " calls.

impl <T> From<T> for Context<T>
{
    fn from(e: T) -> Self {
        Context(e, "".into())
    }
}

impl <T> Context<T> {
    pub fn value(&self) -> &T {
        &self.0
    }

    pub fn context(&self) -> &str {
        self.1.as_str()
    }

    pub fn format_with_context(&self, s: &str) -> String {
        format!("'{}' Context:\n{}", s, self.context())
    }

    fn extend<'a, A>(&'a self, mut other: Context<A>) -> Context<A> {
        other.1 = format!("{}\n{}", other.context(), self.context());
        other
    }
}

impl ElementContext<'_> {
    pub fn element<'a>(&'a self, id: &str) -> ElementResult<'a> {
        let child = self.0.get_child((id, "http://www.w3.org/2005/Atom"));
        child
            .map(|c| self.extend(Context(c, format!("In element '{}'", id))))
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element '{}'. Context:\n{}", id, self.1)))
    }

    pub fn elements(&self, id: &str) -> Context<Vec<&Element>> {
        let items = self.0.children
            .iter()
            .filter_map(|e| match e {
                XMLNode::Element(elem) => Some(elem),
                _ => None,
            })
            .filter(|e| id.match_element(e))
            .collect();
        self.extend(Context(items, format!("In elements '{}'", id)))
    }

    pub fn text(&self) -> Result<Context<std::borrow::Cow<str>>, ParsingError> {
        let text = self.0.get_text()
            .map(|e| self.extend(Context(e, "In text".into())))
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing text. Context:\n{}", self.1)))?;
        Ok(text)
    }

    pub fn attribute(&self, name: &str) -> Result<Context<&String>, ParsingError> {
        self.0.attributes.get(name)
            .map(|s| self.extend(Context(s, format!("In attribute {}", s))))
            .ok_or_else(||
                ParsingError::InvalidXmlStructure(format!(
                    "Missing attribute '{}'. Context:\n{}",
                    name, self.1)))
    }
}

impl TryInto<Url> for Context<&String> {

    type Error= ParsingError;

    fn try_into(self) -> Result<Url, Self::Error> {
        Url::parse(&self.0).map_err(|err| ParsingError::InvalidXmlStructure(format!("Invalid url: {}. Context:\n{}", err, self.1)))
    }
}

impl Context<Vec<&Element>> {

    pub fn find_with_attribute_value(&self, name: &str, value: &str) -> Context<Option<&Element>> {
        fn has_attribute_with_value(element: &Element, name: &str, value: &str) -> bool {
            element.attributes.get(name).map_or(false, |r| r == value)
        }

        let item = self.0
                .iter()
                .find(|e| has_attribute_with_value(e, name, value))
                .map(|e| *e);
        self.extend(Context(item, format!("With attribute {}={}", name, value)))
    }
}

impl Context<Option<&Element>> {
    pub fn into_parsing_result(&self) -> Result<ElementContext, ParsingError> {
        self.0.ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element. Context:\n{}", self.1)))
            .map(|e| self.extend(Context::from(e)))
    }
}