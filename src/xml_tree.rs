use crate::parsing::*;
use xmltree::Element;
use xmltree::XMLNode;
use xmltree::ElementPredicate;
use std::convert::TryInto;
use url::Url;
use std::mem;

pub struct Context<T> {
    value: T,
    context: String
}

pub type ElementContext<'a> = Context<&'a Element>;

type ParsingResult<T> = Result<T, ParsingError>;

type ElementResult<'a> = ParsingResult<ElementContext<'a>>;

impl <T> From<T> for Context<T>
{
    fn from(e: T) -> Self {
        Context::new(e, "".into())
    }
}

impl <T> Context<T> {
    fn new(value: T, context: String) -> Context<T> {
        Context{ value: value, context: context }
    }

    pub fn value_ref(&self) -> &T {
        &self.value
    }

    pub fn value_take(&mut self) -> T
    where T: Default {
        mem::take(&mut self.value)
    }

    pub fn invalid_xml_structure(&self, s: &str) -> ParsingError {
        let text = format!("{} Context:\n{}", s, self.context);
        ParsingError::InvalidXmlStructure(text)
    }

    fn extend<'a, A>(&'a self, mut other: Context<A>) -> Context<A> {
        other.context = format!("{}\n{}", other.context, self.context);
        other
    }
}

impl ElementContext<'_> {
    pub fn element<'a>(&'a self, id: &str) -> ElementResult<'a> {
        let child = self.value.get_child((id, "http://www.w3.org/2005/Atom"));
        child
            .map(|c| self.extend(Context::new(c, format!("In element {}", id))))
            .ok_or_else(|| self.invalid_xml_structure(&format!("Missing element {}", id)))
    }

    pub fn elements(&self, id: &str) -> Context<Vec<&Element>> {
        let items = self.value.children
            .iter()
            .filter_map(|e| match e {
                XMLNode::Element(elem) => Some(elem),
                _ => None,
            })
            .filter(|e| id.match_element(e))
            .collect();
        self.extend(Context::new(items, format!("In elements {}", id)))
    }


    pub fn text(&self) -> Result<Context<std::borrow::Cow<str>>, ParsingError> {
        let text = self.value.get_text()
            .map(|e| self.extend(Context::new(e, "In text".into())))
            .ok_or_else(|| self.invalid_xml_structure("Missing text"))?;
        Ok(text)
    }

    pub fn attribute(&self, name: &str) -> Result<Context<&String>, ParsingError> {
        self.value.attributes.get(name)
            .map(|s| self.extend(Context::new(s, format!("In attribute {}", s))))
            .ok_or_else(|| self.invalid_xml_structure(&format!("Missing attribute {}", name)))
    }
}

impl Context<Vec<&Element>> {
    pub fn map<'a, T, F>(&'a self, f: F) -> Context<Vec<T>>
    where F: Fn(ElementContext<'a>) -> T
    {
        let val = self.value.iter().map(|e| f(self.extend(ElementContext::new(*e, "Being iterated over".into())))).collect();
        self.extend(Context::new(val, "".into()))
    }
}

impl TryInto<Url> for Context<&String> {

    type Error= ParsingError;

    fn try_into(self) -> Result<Url, Self::Error> {
        Url::parse(&self.value).map_err(|err| self.invalid_xml_structure(&format!("Invalid url {}", err)))
    }
}

impl Context<Vec<&Element>> {

    pub fn find_with_attribute_value(&self, name: &str, value: &str) -> Context<Option<&Element>> {
        fn has_attribute_with_value(element: &Element, name: &str, value: &str) -> bool {
            element.attributes.get(name).map_or(false, |r| r == value)
        }

        let item = self.value
                .iter()
                .find(|e| has_attribute_with_value(e, name, value))
                .map(|e| *e);
        self.extend(Context::new(item, format!("With attribute {}={}", name, value)))
    }
}

impl Context<Option<&Element>> {
    pub fn into_parsing_result(&self) -> Result<ElementContext, ParsingError> {
        self.value.ok_or_else(|| self.invalid_xml_structure(&format!("Missing element")))
            .map(|e| self.extend(Context::from(e)))
    }
}