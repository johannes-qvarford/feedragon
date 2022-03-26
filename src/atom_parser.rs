use crate::parsing::*;
use xmltree::Element;
use chrono::prelude::*;

struct AtomParser;

impl Parser for AtomParser {
    fn parse_feed(tree: Element) -> Result<Feed, ParsingError> {
        unimplemented!()
    }
}

impl AtomParser {
    fn parse_entry(&self, atom_entry: Element) -> Result<Entry, ParsingError> {

        let get_child = |id: &str| {
            atom_entry.get_child((id, "http://www.w3.org/2005/Atom"))
                .ok_or_else(|| ParsingError::InvalidXmlStructure(format!("Missing entry element '{}' for {:?}", id, atom_entry)))
        };

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
mod tests {
    use super::*;

    #[test]
    fn invidious_entry_can_be_parsed() {
        let entry_str = r##"
        <entry xmlns:yt="http://www.youtube.com/xml/schemas/2015" xmlns:media="http://search.yahoo.com/mrss/" xmlns="http://www.w3.org/2005/Atom">
    <id>yt:video:be8ZARHsjmc</id>
    <yt:videoId>be8ZARHsjmc</yt:videoId>
    <yt:channelId>UCnyP4sbJVIU9JqHz4l6oQZw</yt:channelId>
    <title>SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨</title>
    <link rel="alternate" href="http://invidious.privacy.qvarford.net/watch?v=be8ZARHsjmc"/>
    <author>
      <name>SmallAnt Clips</name>
      <uri>http://invidious.privacy.qvarford.net/channel/UCnyP4sbJVIU9JqHz4l6oQZw</uri>
    </author>
    <content type="xhtml">
      <div xmlns="http://www.w3.org/1999/xhtml">
        <a href="http://invidious.privacy.qvarford.net/watch?v=be8ZARHsjmc">
          <img src="http://invidious.privacy.qvarford.net/vi/be8ZARHsjmc/mqdefault.jpg"/>
        </a>
      </div>
    </content>
    <published>2022-03-22T07:00:09+00:00</published>
    <updated>2022-03-22T07:26:01+00:00</updated>
    <media:group>
      <media:title>SmallAnt makes a ✨𝘧𝘳𝘪𝘦𝘯𝘥✨</media:title>
      <media:thumbnail url="http://invidious.privacy.qvarford.net/vi/be8ZARHsjmc/mqdefault.jpg" width="320" height="180"/>
    </media:group>
  </entry>
        "##;
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
}