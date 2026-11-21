use super::*;

fn rules() -> &'static RuleSet {
    RuleSet::builtin().expect("the built-in rules are valid")
}

fn hcm() -> DiseaseArchitecture {
    // Whiffin 2017, hypertrophic cardiomyopathy: 1 in 500, MYBPC3 c.1504C>T at most 2%, penetrance 0.5.
    DiseaseArchitecture {
        prevalence: 1.0 / 500.0,
        penetrance: 0.5,
        allelic_contribution: 0.02,
        genetic_contribution: 1.0,
    }
}

fn pcd() -> DiseaseArchitecture {
    // Whiffin 2017, primary ciliary dyskinesia: 1 in 10,000, DNAI1 9% of cases, IVS1+2_3insT 57% of them.
    DiseaseArchitecture {
        prevalence: 1.0 / 10_000.0,
        penetrance: 0.5,
        allelic_contribution: 0.57,
        genetic_contribution: 0.09,
    }
}

fn found(findings: &[PopulationFinding], code: &str) -> Option<Strength> {
    findings.iter().find(|f| f.code == code).and_then(|f| f.reading.as_ref()).map(|r| r.strength)
}

fn read(facts: PopulationFacts) -> Vec<PopulationFinding> {
    rules().read_population(&facts, false).expect("the built-in rules have a population table")
}

#[test]
fn the_maximum_credible_frequency_reproduces_the_papers_worked_examples() {
    let dominant = max_credible_frequency(Inheritance::AutosomalDominant, &hcm()).unwrap();
    assert!((dominant - 4.0e-5).abs() < 1e-12, "HCM: the paper says 4.0e-5, got {dominant}");
    let recessive = max_credible_frequency(Inheritance::AutosomalRecessive, &pcd()).unwrap();
    assert!((recessive - 2.42e-3).abs() < 5e-6, "PCD: the paper says 2.42e-3, got {recessive}");
    // No formula for X-linked or unknown inheritance, and no value outside 0-1.
    assert_eq!(max_credible_frequency(Inheritance::XLinked, &hcm()), None);
    assert_eq!(max_credible_frequency(Inheritance::Unknown, &hcm()), None);
    assert_eq!(
        max_credible_frequency(Inheritance::AutosomalDominant, &DiseaseArchitecture { penetrance: 0.0, ..hcm() }),
        None
    );
    assert_eq!(
        max_credible_frequency(Inheritance::AutosomalDominant, &DiseaseArchitecture { prevalence: 2.0, ..hcm() }),
        None
    );
}

#[test]
fn ba1_needs_more_than_five_percent_in_at_least_two_thousand_alleles() {
    let facts = |frequency, alleles| PopulationFacts {
        frequency: Some(frequency),
        alleles: Some(alleles),
        ..Default::default()
    };
    assert_eq!(found(&read(facts(0.06, 2000)), "BA1"), Some(Strength::StandAlone));
    assert_eq!(found(&read(facts(0.05, 50_000)), "BA1"), None, "0.05 itself is not above 0.05");
    let few = read(facts(0.2, 1999));
    assert_eq!(found(&few, "BA1"), None);
    assert!(few[0].why.contains("at least 2000"), "{}", few[0].why);
    // An expert panel's lower limit replaces 0.05.
    let gene = PopulationFacts { gene_ba1: Some(0.01), ..facts(0.02, 5000) };
    assert_eq!(found(&read(gene), "BA1"), Some(Strength::StandAlone));
}

#[test]
fn a_variant_on_the_exception_list_never_gets_ba1() {
    let facts = PopulationFacts { frequency: Some(0.3), alleles: Some(100_000), ..Default::default() };
    let findings = rules().read_population(&facts, true).unwrap();
    assert_eq!(found(&findings, "BA1"), None);
    assert!(findings[0].why.contains("exception list"));
}

#[test]
fn bs1_compares_the_filtering_frequency_with_the_maximum_credible_one() {
    let facts = |filtering| PopulationFacts {
        frequency: Some(1e-4),
        alleles: Some(100_000),
        filtering_frequency: filtering,
        inheritance: Some(Inheritance::AutosomalDominant),
        architecture: Some(hcm()),
        ..Default::default()
    };
    assert_eq!(found(&read(facts(Some(5e-5))), "BS1"), Some(Strength::Strong));
    assert_eq!(found(&read(facts(Some(4e-5))), "BS1"), Some(Strength::Strong), "at the limit counts, as in the paper");
    assert_eq!(found(&read(facts(Some(3e-5))), "BS1"), None);
    let shown = read(facts(Some(5e-5)));
    assert!(shown[1].why.contains("× ½") && shown[1].why.contains("Whiffin 2017"), "{}", shown[1].why);
    let asked = read(facts(None));
    assert!(asked[1].why.contains("filtering allele frequency"), "{}", asked[1].why);
    // Too common for BS1 also rules out PM2.
    assert_eq!(found(&read(facts(Some(5e-5))), "PM2"), None);
}

#[test]
fn pm2_is_absence_and_for_recessive_disease_only_a_gene_limit_counts() {
    let absent = PopulationFacts { absent: true, ..Default::default() };
    assert_eq!(found(&read(absent), "PM2"), Some(Strength::Supporting));
    let recessive = |gene_pm2| PopulationFacts {
        frequency: Some(5e-5),
        inheritance: Some(Inheritance::AutosomalRecessive),
        gene_pm2,
        ..Default::default()
    };
    let without = read(recessive(None));
    assert_eq!(found(&without, "PM2"), None);
    assert!(without[2].why.contains("no general number"), "{}", without[2].why);
    assert_eq!(found(&read(recessive(Some(1e-4))), "PM2"), Some(Strength::Supporting));
    assert_eq!(found(&read(recessive(Some(1e-5))), "PM2"), None);
    // Present in a dominant disease: not absent, so no PM2.
    let present = PopulationFacts {
        frequency: Some(1e-6),
        inheritance: Some(Inheritance::AutosomalDominant),
        ..Default::default()
    };
    assert_eq!(found(&read(present), "PM2"), None);
}

#[test]
fn a_population_table_that_cannot_be_read_as_written_is_refused() {
    let builtin = super::super::BUILTIN;
    for (old, new) in [
        ("ba1_frequency = 0.05", "ba1_frequency = 5.0"),
        ("ba1_min_alleles = 2000", "ba1_min_alleles = 0"),
        ("bs1_source = \"whiffin2017\"", "bs1_source = \"nowhere\""),
    ] {
        assert!(builtin.contains(old), "{old}");
        assert!(RuleSet::parse(&builtin.replacen(old, new, 1)).is_err(), "{new} must be refused");
    }
}
