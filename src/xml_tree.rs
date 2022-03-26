use crate::parsing::*;
use xmltree::Element;
use xmltree::XMLNode;
use xmltree::ElementPredicate;
use std::convert::TryInto;
use url::Url;

pub struct ValueContext<T>(T, String);

pub type ElementContext<'a> = ValueContext<&'a Element>;

type ElementResult<'a> = Result<ElementContext<'a>, ParsingError>;

impl <T> From<T> for ValueContext<T>
{
    fn from(e: T) -> Self {
        ValueContext(e, "".into())
    }
}

impl <T> ValueContext<T> {
    pub fn value(&self) -> &T {
        &self.0
    }

    pub fn context(&self) -> &str {
        self.1.as_str()
    }

    pub fn format_with_context(&self, s: &str) -> String {
        format!("'{}' Context:\n{}", s, self.context())
    }

    fn extend<'a, A>(&'a self, mut other: ValueContext<A>) -> ValueContext<A> {
        other.1 = format!("{}\n{}", other.context(), self.context());
        other
    }

    fn with_value<U>(self, value: U) -> ValueContext<U> {
        ValueContext(value, self.1)
    }

    fn with_more_context(self, context: &str) -> Self {
        ValueContext(self.0, format!("{}\n{}", context, self.1))
    }
}

impl ElementContext<'_> {
    pub fn element<'a>(&'a self, id: &str) -> ElementResult<'a> {
        let child = self.0.get_child((id, "http://www.w3.org/2005/Atom"));
        child
            .map(|c| self.extend(ValueContext(c, format!("In element '{}'", id))))
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element '{}'. Context:\n{}", id, self.1)))
    }

    pub fn elements(&self, id: &str) -> ValueContext<Vec<&Element>> {
        let items = self.0.children
            .iter()
            .filter_map(|e| match e {
                XMLNode::Element(elem) => Some(elem),
                _ => None,
            })
            .filter(|e| id.match_element(e))
            .collect();
        ValueContext(items, self.1.clone()).with_more_context(&format!("In elements '{}'", id))
    }

    pub fn text(&self) -> Result<ValueContext<std::borrow::Cow<str>>, ParsingError> {
        let text = self.0.get_text()
            .map(|e| ValueContext(e, self.1.clone()).with_more_context("In text".into()))
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing text. Context:\n{}", self.1)))?;
        Ok(text)
    }

    pub fn attribute(&self, name: &str) -> Result<ValueContext<&String>, ParsingError> {
        self.0.attributes.get(name)
            .map(|s| ValueContext(s, self.1.clone()).with_more_context(&format!("In attribute {}", s)))
            .ok_or_else(||
                ParsingError::InvalidXmlStructure(format!(
                    "Missing attribute '{}'. Context:\n{}",
                    name, self.1)))
    }
}

impl TryInto<Url> for ValueContext<&String> {

    type Error= ParsingError;

    fn try_into(self) -> Result<Url, Self::Error> {
        Url::parse(&self.0).map_err(|err| ParsingError::InvalidXmlStructure(format!("Invalid url: {}. Context:\n{}", err, self.1)))
    }
}

impl ValueContext<Vec<&Element>> {

    pub fn find_with_attribute_value(&self, name: &str, value: &str) -> ValueContext<Option<&Element>> {
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
    pub fn into_parsing_result(&self) -> Result<ElementContext, ParsingError> {
        self.0.ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element. Context:\n{}", self.1)))
            .map(|e| self.extend(ValueContext::from(e)))
    }
}