use crate::parsing::*;
use xmltree::Element;
use xmltree::XMLNode;
use xmltree::ElementPredicate;
use chrono::prelude::*;

struct AtomParser;

fn child_elements<'a>(tree: &'a Element, name: &str)
    -> Vec<& 'a Element>
    {
    tree.children
        .iter()
        .filter_map(|e| match e {
            XMLNode::Element(elem) => Some(elem),
            _ => None,
        })
        .filter(|e| name.match_element(e))
        .collect()
}

trait Elementy {
    fn element(&self, id: &str) -> Result<&Element, ParsingError>;
    fn elements(&self, id: &str) -> Vec<&Element>;
    fn text(&self) -> Result<String, ParsingError>;
    fn has_attribute_with_value(&self, name: &str, value: &str) -> bool;
    
}

impl Elementy for Element {
    fn element(&self, id: &str) -> Result<&Element, ParsingError> {
        self.get_child((id, "http://www.w3.org/2005/Atom"))
        .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element '{}' for {:?}", id, self)))
    }

    fn elements(&self, id: &str) -> Vec<&Element> {
        self.children
            .iter()
            .filter_map(|e| match e {
                XMLNode::Element(elem) => Some(elem),
                _ => None,
            })
            .filter(|e| id.match_element(e))
            .collect()
    }

    fn text(&self) -> Result<String, ParsingError> {
        let text = self.get_text()
            .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing text from entry element {:?}", self)))?
            .to_string();
        Ok(text)
    }

    fn has_attribute_with_value(&self, name: &str, value: &str) -> bool {
        self.attributes.get(name).map_or(false, |r| r == value)
    }
}

trait ElementyOption {
    fn into_parsing_result(&self, err_str: &str) -> Result<&Element, ParsingError>;
}

impl ElementyOption for Option<&Element> {
    fn into_parsing_result(&self, err_str: &str) -> Result<&Element, ParsingError> {
        self.ok_or_else(|| { ParsingError::InvalidXmlStructure(format!("Missing element : {}", err_str).into()) })
    }
}

impl Parser for AtomParser {
    fn parse_feed(&self, tree: Element) -> Result<Feed, ParsingError> {
        Ok(Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: tree.element("title")?.text()?,
            link: tree.elements("link")
                .iter()
                .find(|e| e.has_attribute_with_value("rel", "self"))
                .map(|e| *e)
                .into_parsing_result("Missing <link ref='self'> element")?
                .attributes.get("href").unwrap()
                .as_str().try_into().unwrap(),
            title: tree.get_child("title").unwrap().get_text().unwrap().to_string()
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
        let extract_text = |id: &str| -> Result<String, ParsingError> {
            atom_entry.element(id)?.text()
        };

        let extract_attribute = |id: &str, attribute_name: &str| -> Result<String, ParsingError> {
            let child = atom_entry.element(id)?;
            let value = child.attributes.get(attribute_name)
                .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing atribute '{}' from element '{}' in entry {:?}", attribute_name, id, atom_entry)))?
                .clone();
            Ok(value)
        };

        // Make these helper functions more modular, so you say stuff like:
        //  date_time(text(get_child(id)))
        // Or create a builder like:
        //  get_child(id).text().date_time()
        // But that seems overkill.
        // Also, make error messages add context.
        // Maybe that reqires get_child to call text, that calls date_time, passing each as a callback argument.
        let extract_date_time = |id: &str| -> Result<DateTime<Utc>, ParsingError> {
            let text = atom_entry.element(id)?.text()?;
            let date_time_with_offset = DateTime::parse_from_rfc3339(&text)
                .map_err(|_dt_err| ParsingError::InvalidXmlStructure(
                    format!("Invalid rfc 3339 date time '{}' from element '{}' in entry element {:?}", text, id, atom_entry)))?;
            return Ok(DateTime::from(date_time_with_offset))
        };

        Ok(Entry {
            title: extract_text("title")?,
            id: extract_text("id")?,
            link: extract_attribute("link", "href")?,
            summary: extract_text("title")?,
            // 2022-03-22T07:00:09+00:00
            updated: extract_date_time("updated")?
            //DateTime::from_utc(NaiveDate::from_ymd(2022, 3, 22).and_hms(7, 0, 0), Utc{}),
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
            // 2022-03-22T07:00:09+00:00
            updated: DateTime::parse_from_rfc3339("2022-03-22T07:26:01+00:00").unwrap().into(),
        };
        assert_eq!(expected, entry);
    }

    #[test]
    fn invalid_entry_gives_detailed_error() {
        let entry_str = std::fs::read_to_string("src/example_atom_entry.xml")
            .expect("Expected example file to exist.");
        let mut atom_entry = Element::parse(entry_str.as_bytes()).unwrap();
        let attr = atom_entry.get_mut_child("link").unwrap().attributes.remove("href");
        let parser = AtomParser{};

        let entry = parser.parse_entry(atom_entry.clone());

        assert_eq!(Err(ParsingError::InvalidXmlStructure(
            format!("Missing atribute 'href' from element 'link' in entry {:?}", atom_entry))), entry)
    }
}