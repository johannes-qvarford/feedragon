use crate::parsing::*;
use xmltree::Element;
use xmltree::XMLNode;
use xmltree::ElementPredicate;
use chrono::prelude::*;
use url::Url;

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

fn get_child<'a>(atom_entry: &'a Element, id: &str) -> Result<&'a Element, ParsingError> {
    atom_entry.get_child((id, "http://www.w3.org/2005/Atom"))
        .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing element '{}' for {:?}", id, atom_entry)))
}

impl Parser for AtomParser {
    fn parse_feed(&self, tree: Element) -> Result<Feed, ParsingError> {

        Ok(Feed {
            author_name: "Unknown".into(),
            entries: vec![],
            id: tree.get_child("title").unwrap().get_text().unwrap().to_string(),
            link: child_elements(&tree, "link")
                .iter()
                .find(|e| e.attributes.get("rel").map_or(false, |r| r == "self")).unwrap()
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

        let get_child = |id: &_| get_child(&atom_entry, id);

        let extract_text = |id: &str| -> Result<String, ParsingError> {
            let child = get_child(id)?;
            let text = child.get_text()
                .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing text from entry element {:?}", atom_entry)))?
                .to_string();
            Ok(text)
        };

        let extract_attribute = |id: &str, attribute_name: &str| -> Result<String, ParsingError> {
            let child = get_child(id)?;
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
            let text = extract_text(id)?;
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