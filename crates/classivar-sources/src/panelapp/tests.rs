use super::*;

/// Shaped like a real PanelApp response for BRCA1, with fields trimmed.
/// The first two entries follow the real answer of 2026-11-03; the third is invented to cover level 1.
const SAMPLE: &str = include_str!("../../tests/fixtures/panelapp_brca1_sample.json");

#[test]
fn parses_a_response_and_ignores_fields_it_does_not_need() {
    let page = parse_page(SAMPLE, "BRCA1").unwrap();
    assert_eq!(page.next, None);
    assert_eq!(page.entries.len(), 3);
    assert_eq!(page.entries[0].panel_name, "Familial breast cancer");
    assert_eq!(page.entries[0].panel_version, "1.28");
    assert_eq!(page.entries[0].confidence, Confidence::Green);
    assert_eq!(page.entries[0].phenotypes, vec!["{Breast-ovarian cancer, familial, 1}, OMIM:604370"]);
}

#[test]
fn entries_are_sorted_best_confidence_first() {
    let gene = finish("BRCA1".into(), parse_page(SAMPLE, "BRCA1").unwrap().entries);
    let order: Vec<_> = gene.entries.iter().map(|e| e.confidence).collect();
    assert_eq!(order, vec![Confidence::Green, Confidence::Amber, Confidence::Red]);
    assert_eq!(gene.count_at(Confidence::Green), 1);
}

#[test]
fn an_unknown_confidence_level_is_an_error_not_a_guess() {
    let body = SAMPLE.replace("\"confidence_level\": \"3\"", "\"confidence_level\": \"7\"");
    assert!(matches!(parse_page(&body, "BRCA1"), Err(SourceError::Unexpected(_))));
}

#[test]
fn the_page_count_is_read_so_a_partial_answer_can_be_noticed() {
    let page = parse_page(SAMPLE, "BRCA1").unwrap();
    assert_eq!((page.count, page.received), (3, 3));
    // Entries for other genes still count as received.
    let other = parse_page(SAMPLE, "BRCA2").unwrap();
    assert_eq!((other.count, other.received, other.entries.len()), (3, 3, 0));
}

/// A made-up page with `n` BRCA1 entries.
fn page(count: u64, next: Option<&str>, n: u64) -> String {
    let entry = r#"{"entity_name":"BRCA1","confidence_level":"3","panel":{"id":1,"name":"P","version":"1.0"}}"#;
    let results = vec![entry; n as usize].join(",");
    let next = next.map_or("null".to_string(), |u| format!("\"{u}\""));
    format!(r#"{{"count":{count},"next":{next},"results":[{results}]}}"#)
}

const NEXT: &str = "https://panelapp.genomicsengland.co.uk/api/v1/genes/?entity_name=BRCA1&page=2";

#[test]
fn two_pages_are_followed_and_joined() {
    let mut asked = Vec::new();
    let entries = collect_pages(Instance::England, "BRCA1", |url, query| {
        asked.push((url.to_string(), query.len()));
        Ok(if asked.len() == 1 { page(3, Some(NEXT), 2) } else { page(3, None, 1) })
    })
    .unwrap();
    assert_eq!(entries.len(), 3);
    // The first request carries the search; the second uses the next link as it is.
    assert_eq!(asked, vec![(Instance::England.url().to_string(), 2), (NEXT.to_string(), 0)]);
}

#[test]
fn a_truncated_answer_is_an_error_not_a_shorter_list() {
    let result = collect_pages(Instance::England, "BRCA1", |_, _| Ok(page(5, None, 3)));
    assert!(matches!(result, Err(SourceError::Unexpected(m)) if m.contains("incomplete")));
}

#[test]
fn an_endless_series_of_pages_stops_with_an_error() {
    let result = collect_pages(Instance::England, "BRCA1", |_, _| Ok(page(99, Some(NEXT), 1)));
    assert!(matches!(result, Err(SourceError::Unexpected(m)) if m.contains("pages")));
}

#[test]
fn a_next_link_to_another_host_is_refused_before_it_is_fetched() {
    let mut calls = 0;
    let result = collect_pages(Instance::England, "BRCA1", |_, _| {
        calls += 1;
        Ok(page(2, Some("https://example.com/api/v1/genes/?page=2"), 1))
    });
    assert!(matches!(result, Err(SourceError::Unexpected(_))));
    assert_eq!(calls, 1);
}

#[test]
fn next_page_links_must_stay_on_the_same_service() {
    let ok = "https://panelapp.genomicsengland.co.uk/api/v1/genes/?entity_name=BRCA1&page=2";
    assert!(check_next_link(Instance::England, ok).is_ok());
    assert!(check_next_link(Instance::England, "http://panelapp.genomicsengland.co.uk/api/v1/genes/").is_err());
    assert!(check_next_link(Instance::England, "https://panelapp.genomicsengland.co.uk.example.com/x").is_err());
    assert!(check_next_link(Instance::England, "https://panelapp-aus.org/api/v1/genes/").is_err());
    assert!(check_next_link(Instance::Australia, "https://panelapp-aus.org/api/v1/genes/?page=2").is_ok());
}

#[test]
fn garbage_is_an_error() {
    assert!(matches!(parse_page("<html>maintenance</html>", "BRCA1"), Err(SourceError::Unexpected(_))));
}

#[test]
fn entries_for_other_genes_are_dropped() {
    assert!(parse_page(SAMPLE, "BRCA2").unwrap().entries.is_empty());
}

/// Needs the internet. Run on purpose with: cargo test -p classivar-sources -- --ignored
#[test]
#[ignore]
fn live_brca1_is_on_a_green_panel_in_both_services() {
    for instance in [Instance::England, Instance::Australia] {
        let gene = lookup_gene(instance, "BRCA1").unwrap();
        assert_eq!(gene.source, instance.source_name());
        assert!(gene.value.count_at(Confidence::Green) >= 1, "{instance:?}");
    }
}

#[test]
#[ignore]
fn live_unknown_gene_is_empty_not_an_error() {
    let gene = lookup_gene(Instance::England, "NOTAGENE1").unwrap();
    assert!(gene.value.entries.is_empty());
}
