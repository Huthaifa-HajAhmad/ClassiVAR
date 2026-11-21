//! A variant in a gene of uncertain significance (Richards 2015, "Evaluating and
//! Reporting Variants in Genes of Uncertain Significance") is classified VUS.

use super::*;

fn in_doubtful_gene(list: &[(&str, Strength)]) -> Assessment {
    apply(list).in_gene_of_uncertain_significance()
}

#[test]
fn pathogenic_points_give_vus_when_the_gene_disease_link_is_not_established() {
    // 4 + 4 + 1 + 1 = 10 points: Pathogenic.
    let evidence = [("PS3", Strong), ("PS4", Strong), ("PM2", Supporting), ("PP3", Supporting)];
    assert_eq!(apply(&evidence).class(), Some(Class::Pathogenic));
    let a = in_doubtful_gene(&evidence);
    assert_eq!(a.class(), Some(Class::Vus), "the source says such variants are always VUS");
    assert_eq!(a.points_class, Class::Pathogenic, "the points themselves are not changed");
    assert_eq!(a.total_points, 10, "same total");
    assert!(a.gene_of_uncertain_significance);
}

#[test]
fn no_hint_points_to_a_class_the_gene_link_forbids() {
    // 7 points: Likely pathogenic by points, VUS in a doubtful gene. Neither the
    // way up (to Pathogenic) nor a way down from Likely pathogenic makes sense.
    let a = in_doubtful_gene(&[("PS3", Strong), ("PM1", Moderate), ("PP3", Supporting)]);
    assert_eq!(a.class(), Some(Class::Vus));
    assert_eq!(a.to_upgrade, None);
    assert_eq!(a.to_downgrade, None);
    // At 3 points the class is VUS either way, but Likely pathogenic is out of reach.
    let a = in_doubtful_gene(&[("PM5", Moderate), ("PP3", Supporting)]);
    assert_eq!(a.class(), Some(Class::Vus));
    assert_eq!(a.to_upgrade, None);
    assert!(a.to_downgrade.is_some(), "benign evidence still moves it down");
}

#[test]
fn benign_and_withheld_results_are_left_as_they_are() {
    // The source is about candidate variants being called pathogenic; a benign
    // result is not lifted to VUS (a decision recorded for the human reviewer).
    assert_eq!(in_doubtful_gene(&[("BS1", Strong), ("BP4", Supporting)]).class(), Some(Class::LikelyBenign));
    assert_eq!(in_doubtful_gene(&[("BA1", StandAlone)]).class(), Some(Class::Benign));
    // A withheld class stays withheld: the limit never invents a class.
    assert_eq!(in_doubtful_gene(&[("PP3", Supporting), ("BP4", Supporting)]).class(), None);
    assert_eq!(in_doubtful_gene(&[]).class(), None);
}
