use chrono::prelude::*;
use url::Url;

#[derive(Debug, PartialEq, Clone)]
pub struct Entry {
    pub title: String,
    pub link: String,
    pub id: String,
    pub updated: DateTime<Utc>,
    pub summary: String,
}

#[derive(Debug, PartialEq)]
pub struct Feed {
    pub title: String,
    pub link: Url,
    pub author_name: String,
    pub id: String,
    pub entries: Vec<Entry>,
}

pub fn merge_feeds(id: String, link: Url, feeds: Vec<Feed>) -> Feed {
    let titles = feeds
        .iter()
        .map(|feed| feed.title.as_str())
        .collect::<Vec<_>>()
        .join(" + ");

    let mut entries: Vec<_> = feeds.into_iter().flat_map(|feed| feed.entries).collect();
    // latest entry is first.
    entries.sort_by(|a, b| b.updated.cmp(&a.updated));

    Feed {
        title: titles,
        author_name: "Unknown".into(),
        id,
        link,
        entries,
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn latest_entries_are_first() {
        let feed1 = Feed {
            author_name: "James".into(),
            id: "James ID".into(),
            link: "https://james.com/rss".try_into().unwrap(),
            title: "James Feed".into(),
            entries: vec![
                Entry {
                    id: "James Post 2".into(),
                    link: "https://james.com/posts/2".try_into().unwrap(),
                    title: "More thoughs on posting".into(),
                    summary: "Posting keeps being nice.".into(),
                    updated: DateTime::parse_from_rfc3339("2022-01-01T00:00:00+00:00")
                        .unwrap()
                        .into(),
                },
                Entry {
                    id: "James Post 1".into(),
                    link: "https://james.com/posts/1".try_into().unwrap(),
                    title: "Thinking about posting.".into(),
                    summary: "Posting is really nice.".into(),
                    updated: DateTime::parse_from_rfc3339("2020-01-01T00:00:00+00:00")
                        .unwrap()
                        .into(),
                },
            ],
        };
        let feed2 = Feed {
            author_name: "Jessica".into(),
            id: "Jessica ID".into(),
            link: "https://jessica.com/rss".try_into().unwrap(),
            title: "Jessica Feed".into(),
            entries: vec![
                Entry {
                    id: "Jessica Post 2".into(),
                    link: "https://jessica.com/posts/2".try_into().unwrap(),
                    title: "I made a post two years ago.".into(),
                    summary: "Posting has not improved in two years.".into(),
                    updated: DateTime::parse_from_rfc3339("2021-01-01T00:00:00+00:00")
                        .unwrap()
                        .into(),
                },
                Entry {
                    id: "Jessica Post 1".into(),
                    link: "https://jessica.com/posts/1".try_into().unwrap(),
                    title: "Posting should improve.".into(),
                    summary: "It can only get better!".into(),
                    updated: DateTime::parse_from_rfc3339("2019-01-01T00:00:00+00:00")
                        .unwrap()
                        .into(),
                },
            ],
        };

        let merged = merge_feeds(
            "Friends".into(),
            "https://friends.com/rss".try_into().unwrap(),
            vec![feed1, feed2],
        );

        let post_ids: Vec<&str> = merged
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(
            post_ids,
            vec![
                "James Post 2",
                "Jessica Post 2",
                "James Post 1",
                "Jessica Post 1"
            ]
        );

        assert_eq!(merged.title, "James Feed + Jessica Feed")
    }
}
