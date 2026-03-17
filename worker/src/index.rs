use std::collections::HashMap;

/// A single item in the search index.
#[derive(Clone, Debug)]
pub struct IndexItem {
    pub id: u64,
    /// Text fields for full-text search (e.g. title, description).
    pub text_fields: Vec<String>,
    /// Numeric fields for range filtering (e.g. price, rating).
    pub numeric_fields: HashMap<String, f64>,
    /// Optional coordinates for geographic filtering.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
}

/// Tokenized representation for fast text search.
#[derive(Clone, Debug)]
struct TokenizedItem {
    pub item: IndexItem,
    pub tokens: Vec<String>,
}

/// Generic in-memory search index with text search, numeric filters,
/// coordinate filtering, and pagination.
#[derive(Clone, Debug)]
pub struct SearchIndex {
    items: Vec<TokenizedItem>,
}

/// Query filters for searching the index.
#[derive(Debug, Default)]
pub struct SearchQuery {
    /// Text query — matched against all text fields (tokenized, case-insensitive).
    pub text: Option<String>,
    /// Numeric range filters: field_name -> (min, max). Both bounds are inclusive.
    pub numeric_filters: HashMap<String, (f64, f64)>,
    /// Geographic bounding box: (min_lat, max_lat, min_lon, max_lon).
    pub bbox: Option<(f64, f64, f64, f64)>,
    /// Pagination limit (default 20, max 100).
    pub limit: usize,
    /// Pagination offset (default 0).
    pub offset: usize,
}

/// Search result with relevance score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: u64,
    pub score: f64,
}

impl SearchIndex {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Number of items in the index.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Insert or update an item in the index.
    pub fn insert(&mut self, item: IndexItem) {
        // Remove existing item with same id
        self.items.retain(|t| t.item.id != item.id);

        let tokens = tokenize_item(&item);
        self.items.push(TokenizedItem { item, tokens });
    }

    /// Remove an item by id.
    pub fn remove(&mut self, id: u64) {
        self.items.retain(|t| t.item.id != id);
    }

    /// Search the index. Returns results sorted by relevance (highest first).
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let limit = query.limit.min(100).max(1);
        let query_tokens = query.text.as_ref().map(|t| tokenize(t));

        let mut results: Vec<SearchResult> = self
            .items
            .iter()
            .filter_map(|entry| {
                // Apply numeric filters
                for (field, (min, max)) in &query.numeric_filters {
                    match entry.item.numeric_fields.get(field) {
                        Some(val) if *val >= *min && *val <= *max => {}
                        Some(_) => return None,
                        None => return None,
                    }
                }

                // Apply bounding box filter
                if let Some((min_lat, max_lat, min_lon, max_lon)) = query.bbox {
                    match (entry.item.lat, entry.item.lon) {
                        (Some(lat), Some(lon))
                            if lat >= min_lat
                                && lat <= max_lat
                                && lon >= min_lon
                                && lon <= max_lon => {}
                        _ => return None,
                    }
                }

                // Calculate text relevance score
                let score = match &query_tokens {
                    Some(qt) if !qt.is_empty() => {
                        let mut hits = 0.0;
                        for q_token in qt {
                            for i_token in &entry.tokens {
                                if i_token.contains(q_token) {
                                    hits += 1.0;
                                    // Exact match bonus
                                    if i_token == q_token {
                                        hits += 0.5;
                                    }
                                }
                            }
                        }
                        if hits == 0.0 {
                            return None; // No text match
                        }
                        hits / qt.len() as f64
                    }
                    _ => 1.0, // No text query — all items match equally
                };

                Some(SearchResult {
                    id: entry.item.id,
                    score,
                })
            })
            .collect();

        // Sort by score descending, then by id descending (newest first)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.id.cmp(&a.id))
        });

        // Apply pagination
        results.into_iter().skip(query.offset).take(limit).collect()
    }
}

/// Tokenize a string into lowercase words.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Tokenize all text fields of an item.
fn tokenize_item(item: &IndexItem) -> Vec<String> {
    let mut tokens = Vec::new();
    for field in &item.text_fields {
        tokens.extend(tokenize(field));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(id: u64, texts: Vec<&str>) -> IndexItem {
        IndexItem {
            id,
            text_fields: texts.into_iter().map(String::from).collect(),
            numeric_fields: HashMap::new(),
            lat: None,
            lon: None,
        }
    }

    #[test]
    fn test_text_search() {
        let mut idx = SearchIndex::new();
        idx.insert(make_item(1, vec!["Hello world"]));
        idx.insert(make_item(2, vec!["Goodbye world"]));
        idx.insert(make_item(3, vec!["Something else"]));

        let results = idx.search(&SearchQuery {
            text: Some("world".into()),
            limit: 10,
            ..Default::default()
        });
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_numeric_filter() {
        let mut idx = SearchIndex::new();
        let mut item = make_item(1, vec!["Item one"]);
        item.numeric_fields.insert("price".into(), 50.0);
        idx.insert(item);

        let mut item = make_item(2, vec!["Item two"]);
        item.numeric_fields.insert("price".into(), 150.0);
        idx.insert(item);

        let mut filters = HashMap::new();
        filters.insert("price".into(), (0.0, 100.0));

        let results = idx.search(&SearchQuery {
            text: None,
            numeric_filters: filters,
            limit: 10,
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_pagination() {
        let mut idx = SearchIndex::new();
        for i in 1..=10 {
            idx.insert(make_item(i, vec!["test"]));
        }

        let results = idx.search(&SearchQuery {
            text: None,
            limit: 3,
            offset: 2,
            ..Default::default()
        });
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_bbox_filter() {
        let mut idx = SearchIndex::new();
        let mut item = make_item(1, vec!["Stockholm"]);
        item.lat = Some(59.33);
        item.lon = Some(18.07);
        idx.insert(item);

        let mut item = make_item(2, vec!["London"]);
        item.lat = Some(51.51);
        item.lon = Some(-0.13);
        idx.insert(item);

        let results = idx.search(&SearchQuery {
            text: None,
            bbox: Some((55.0, 65.0, 10.0, 25.0)), // Scandinavia
            limit: 10,
            ..Default::default()
        });
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }
}
