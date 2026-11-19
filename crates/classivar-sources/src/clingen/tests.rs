use super::*;

const HEADER: &str = "\"GENE SYMBOL\",\"GENE ID (HGNC)\",\"DISEASE LABEL\",\"DISEASE ID (MONDO)\",\"MOI\",\"SOP\",\"CLASSIFICATION\",\"ONLINE REPORT\",\"CLASSIFICATION DATE\",\"GCEP\"\n";

/// The real file layout (title lines, header, separator rows), shortened. Rows follow the real BRCA1 answer of 2026-11-03.
const SAMPLE: &str = "\"CLINGEN GENE DISEASE VALIDITY CURATIONS\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"\n\
\"FILE CREATED: 2026-11-03\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\",\"\"\n\
\"+++++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"++++++++++++++++++\",\"+++++++++\",\"+++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"+++++++++++++++++++\",\"+++++++++++++++++++\"\n\
\"GENE SYMBOL\",\"GENE ID (HGNC)\",\"DISEASE LABEL\",\"DISEASE ID (MONDO)\",\"MOI\",\"SOP\",\"CLASSIFICATION\",\"ONLINE REPORT\",\"CLASSIFICATION DATE\",\"GCEP\"\n\
\"+++++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"++++++++++++++++++\",\"+++++++++\",\"+++++++++\",\"++++++++++++++\",\"+++++++++++++\",\"+++++++++++++++++++\",\"+++++++++++++++++++\"\n\
\"BRCA1\",\"HGNC:1100\",\"Fanconi anemia, complementation group S\",\"MONDO:0054748\",\"AR\",\"SOP7\",\"Definitive\",\"https://example.org/r1\",\"2020-05-14T00:31:37.538Z\",\"Hereditary Cancer Gene Curation Expert Panel\"\n\
\"BRCA2\",\"HGNC:1101\",\"BRCA2-related cancer predisposition\",\"MONDO:0700269\",\"AD\",\"SOP10\",\"Definitive\",\"https://example.org/r2\",\"2024-08-29T17:00:00.000Z\",\"Hereditary Cancer Gene Curation Expert Panel\"\n\
\"BRCA1\",\"HGNC:1100\",\"An invented disease for this test\",\"MONDO:0000001\",\"AD\",\"SOP10\",\"Limited\",\"https://example.org/r3\",\"2023-01-02T00:00:00.000Z\",\"Test panel\"\n\
\"BRCA1\",\"HGNC:1100\",\"BRCA1-related cancer predisposition\",\"MONDO:0700268\",\"AD\",\"SOP10\",\"Definitive\",\"https://example.org/r4\",\"2024-08-29T17:00:00.000Z\",\"Hereditary Cancer Gene Curation Expert Panel\"\n";

fn gene(rows: &[(String, Curation)], symbol: &str) -> Vec<Curation> {
    super::curations_for(rows, symbol)
}

#[test]
fn picks_out_one_gene_and_sorts_strongest_first() {
    let rows = parse_file(SAMPLE).unwrap();
    assert_eq!(rows.len(), 4);
    let brca1 = gene(&rows, "BRCA1");
    assert_eq!(brca1.len(), 3);
    assert_eq!(brca1[0].disease, "BRCA1-related cancer predisposition");
    assert_eq!(brca1[0].classification, "Definitive");
    assert_eq!(brca1[0].inheritance, "AD");
    assert_eq!(brca1[0].classified_on, "2024-08-29");
    assert_eq!(brca1[2].classification, "Limited");
}

#[test]
fn a_gene_that_is_not_in_the_file_gives_an_empty_list() {
    assert!(gene(&parse_file(SAMPLE).unwrap(), "TP53").is_empty());
}

#[test]
fn columns_are_read_by_name_so_a_reordered_file_still_works() {
    // The same data with the CLASSIFICATION and MOI columns swapped.
    let swapped = SAMPLE
        .replace("\"MOI\",\"SOP\",\"CLASSIFICATION\"", "\"CLASSIFICATION\",\"SOP\",\"MOI\"")
        .replace("\"AD\",\"SOP10\",\"Limited\"", "\"Limited\",\"SOP10\",\"AD\"")
        .replace("\"AD\",\"SOP10\",\"Definitive\"", "\"Definitive\",\"SOP10\",\"AD\"")
        .replace("\"AR\",\"SOP7\",\"Definitive\"", "\"Definitive\",\"SOP7\",\"AR\"");
    let rows = parse_file(&swapped).unwrap();
    let brca1 = gene(&rows, "BRCA1");
    assert_eq!(brca1[0].classification, "Definitive");
    assert_eq!(brca1[0].inheritance, "AD");
}

#[test]
fn a_header_with_a_missing_column_is_an_error() {
    let broken = SAMPLE.replace("\"CLASSIFICATION DATE\"", "\"DATE\"");
    assert!(matches!(parse_file(&broken), Err(SourceError::Unexpected(m)) if m.contains("CLASSIFICATION DATE")));
}

#[test]
fn a_file_without_the_header_row_is_an_error_not_an_empty_answer() {
    assert!(matches!(parse_file("<html>maintenance</html>"), Err(SourceError::Unexpected(_))));
}

#[test]
fn a_header_with_no_data_rows_is_an_error() {
    assert!(matches!(parse_file(HEADER), Err(SourceError::Unexpected(_))));
}

#[test]
fn a_footer_row_is_skipped_not_an_error() {
    // A spacer/footer line of empty cells used to abort the whole file, which
    // made every gene lookup fail until the file changed.
    let with_footer = format!("{SAMPLE},,,,,,,,,\n");
    let rows = parse_file(&with_footer).unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(gene(&rows, "BRCA1").len(), 3);
}

#[test]
fn an_unknown_rating_is_an_error_not_the_weakest_rating() {
    let new_rating = SAMPLE.replace("\"Limited\"", "\"Super Definitive\"");
    assert!(matches!(parse_file(&new_rating), Err(SourceError::Unexpected(m)) if m.contains("Super Definitive")));
}

#[test]
fn a_date_that_is_not_a_date_is_an_error() {
    let bad = SAMPLE.replace("2020-05-14T00:31:37.538Z", "unknown");
    assert!(matches!(parse_file(&bad), Err(SourceError::Unexpected(m)) if m.contains("not a date")));
}

#[test]
fn a_date_with_an_impossible_month_or_day_is_an_error_not_a_display_value() {
    let month = SAMPLE.replace("2020-05-14T00:31:37.538Z", "2020-99-14T00:31:37.538Z");
    assert!(matches!(parse_file(&month), Err(SourceError::Unexpected(_))));
    let day = SAMPLE.replace("2020-05-14T00:31:37.538Z", "2020-05-39T00:31:37.538Z");
    assert!(matches!(parse_file(&day), Err(SourceError::Unexpected(_))));
    let feb = SAMPLE.replace("2020-05-14T00:31:37.538Z", "2020-02-31T00:31:37.538Z");
    assert!(matches!(parse_file(&feb), Err(SourceError::Unexpected(_))), "31 February does not exist");
}

#[test]
fn a_short_row_is_an_error() {
    let short = format!("{HEADER}\"BRCA1\",\"HGNC:1100\"\n");
    assert!(matches!(parse_file(&short), Err(SourceError::Unexpected(_))));
}

/// Needs the internet. Run on purpose with: cargo test -p classivar-sources -- --ignored
#[test]
#[ignore]
fn live_brca1_has_a_definitive_rating() {
    let gene = lookup_gene("BRCA1").unwrap();
    assert!(gene.value.curations.iter().any(|c| c.classification == "Definitive"));
}
