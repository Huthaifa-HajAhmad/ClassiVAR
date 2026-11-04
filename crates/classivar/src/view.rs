//! The main screen. Two paths:
//! - New variant: tick the criteria you apply; see points, class and why.
//! - Reported variant: enter what earlier records used, apply today's evidence,
//!   and see what changed and whether the class holds.
//!
//! The curator decides every criterion; the screen only adds up and explains.

use std::collections::BTreeMap;

use classivar_core::{
    Applied, Assessment, Category, Class, Criterion, Direction, Reason, Reevaluation, RuleSet, Strength, Submission,
    Verdict, Warning,
};
use classivar_sources::{
    Fetched, SourceError,
    clingen::{self, ClinGenGene},
    g2p::{self, G2pGene},
    panelapp::{self, Confidence, Instance, PanelAppGene},
};
use gpui_kit::component::{
    ActiveTheme, Selectable, Sizable,
    button::Button,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Width of one point in the evidence lanes.
const POINT_WIDTH: f32 = 36.0;
/// Width of one point on the points scale, which runs from -8 to +12.
const SCALE_UNIT: f32 = 30.0;
const SCALE_MIN: i32 = -8;
const SCALE_MAX: i32 = 12;

fn pathogenic_colour() -> Rgba {
    rgb(0xe34948)
}

fn benign_colour() -> Rgba {
    rgb(0x2a78d6)
}

fn direction_colour(direction: Direction) -> Rgba {
    match direction {
        Direction::Pathogenic => pathogenic_colour(),
        Direction::Benign => benign_colour(),
    }
}

fn class_fill(class: Class) -> Rgba {
    match class {
        Class::Pathogenic => pathogenic_colour(),
        Class::LikelyPathogenic => rgb(0xf6c9c8),
        Class::Vus => rgb(0xe6e5e0),
        Class::LikelyBenign => rgb(0xcde2fb),
        Class::Benign => benign_colour(),
    }
}

fn class_ink(class: Class) -> Rgba {
    if matches!(class, Class::Pathogenic | Class::Benign) { rgb(0xffffff) } else { rgb(0x0b0b0b) }
}

fn plural(n: i32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// How many detail lines a source card shows before saying "and N more".
const CARD_LINES: usize = 3;

fn and_more(mut lines: Vec<String>, total: usize) -> Vec<String> {
    if total > CARD_LINES {
        lines.push(format!("…and {} more.", total - CARD_LINES));
    }
    lines
}

fn describe_panelapp(found: &Fetched<PanelAppGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.entries.is_empty() {
        return (
            format!("{} is on no panel. That does not prove it is unrelated to disease.", gene.symbol),
            Vec::new(),
        );
    }
    let headline = format!(
        "{} is on {} panel{}: {} green, {} amber, {} red.",
        gene.symbol,
        gene.entries.len(),
        plural(gene.entries.len() as i32),
        gene.count_at(Confidence::Green),
        gene.count_at(Confidence::Amber),
        gene.count_at(Confidence::Red),
    );
    let lines = gene
        .entries
        .iter()
        .take(CARD_LINES)
        .map(|e| {
            format!("• {} (v{}): {} · {}", e.panel_name, e.panel_version, e.confidence.label(), e.mode_of_inheritance)
        })
        .collect();
    (headline, and_more(lines, gene.entries.len()))
}

fn describe_g2p(found: &Fetched<G2pGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.records.is_empty() {
        return (format!("G2P has no record for {}.", gene.symbol), Vec::new());
    }
    let headline = format!("{} gene-disease record{}.", gene.records.len(), plural(gene.records.len() as i32));
    let lines = gene
        .records
        .iter()
        .take(CARD_LINES)
        .map(|r| {
            format!("• {}: {} · {} · {}", r.disease, r.confidence, r.genotype.replace('_', " "), r.molecular_mechanism)
        })
        .collect();
    (headline, and_more(lines, gene.records.len()))
}

fn describe_clingen(found: &Fetched<ClinGenGene>) -> (String, Vec<String>) {
    let gene = &found.value;
    if gene.curations.is_empty() {
        return (format!("ClinGen has not rated {}.", gene.symbol), Vec::new());
    }
    let headline = format!("{} gene-disease rating{}.", gene.curations.len(), plural(gene.curations.len() as i32));
    let lines = gene
        .curations
        .iter()
        .take(CARD_LINES)
        .map(|c| {
            format!(
                "• {} ({}): {} · {} · {}",
                c.disease, c.inheritance, c.classification, c.classified_on, c.expert_panel
            )
        })
        .collect();
    (headline, and_more(lines, gene.curations.len()))
}

type Evidence = BTreeMap<String, Strength>;

fn to_applied(evidence: &Evidence) -> Vec<Applied> {
    evidence.iter().map(|(code, strength)| Applied { code: code.clone(), strength: *strength }).collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    NewVariant,
    ReportedVariant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Editing {
    Today,
    Record(usize),
}

/// What an earlier record (for example a lab in ClinVar) said and used.
struct Record {
    label: String,
    year: u16,
    class: Class,
    evidence: Evidence,
}

/// Where one lookup stands.
enum Lookup<T> {
    Idle,
    Loading,
    Done(Fetched<T>),
    /// The lookup did not work. This is never shown as "nothing found".
    Failed(String),
}

/// What the four gene sources said about the gene the curator asked about.
struct GeneChecks {
    #[allow(dead_code)] // kept for the saved-case file that comes later
    symbol: String,
    england: Lookup<PanelAppGene>,
    australia: Lookup<PanelAppGene>,
    g2p: Lookup<G2pGene>,
    clingen: Lookup<ClinGenGene>,
}

impl GeneChecks {
    fn idle() -> Self {
        Self {
            symbol: String::new(),
            england: Lookup::Idle,
            australia: Lookup::Idle,
            g2p: Lookup::Idle,
            clingen: Lookup::Idle,
        }
    }
}

/// The curator's own answer to: does this gene cause this disease?
/// The program only suggests; this is the curator's decision.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneLink {
    Unconfirmed,
    Established,
    NotEstablished,
}

pub struct Workbench {
    rules: &'static RuleSet,
    mode: Mode,
    /// The curator's own evidence, today.
    today: Evidence,
    records: Vec<Record>,
    /// Which evidence set the criteria list is editing.
    editing: Editing,
    /// When on, every criterion shows its plain-language rule and source.
    learning: bool,
    gene_input: Entity<InputState>,
    gene: GeneChecks,
    /// Whether the four source cards are shown. They are tall, so the curator can hide them.
    gene_cards_open: bool,
    gene_link: GeneLink,
    /// Counts lookups, so a slow old answer cannot replace a newer one.
    lookup_id: u64,
    _lookup_tasks: Vec<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl Workbench {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let gene_input = cx.new(|cx| InputState::new(window, cx).placeholder("Gene symbol, e.g. BRCA1"));
        let on_enter = cx.subscribe_in(&gene_input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::PressEnter { .. }) {
                this.check_gene(cx);
            }
        });
        Self {
            rules: RuleSet::builtin(),
            mode: Mode::NewVariant,
            today: Evidence::new(),
            records: Vec::new(),
            editing: Editing::Today,
            learning: true,
            gene_input,
            gene: GeneChecks::idle(),
            gene_cards_open: true,
            gene_link: GeneLink::Unconfirmed,
            lookup_id: 0,
            _lookup_tasks: Vec::new(),
            _subscriptions: vec![on_enter],
        }
    }

    /// Ask all four gene sources about the gene in the text box. Each network call
    /// runs in the background and fills in its own card when it arrives, so one slow
    /// or failing source does not hold up the others.
    fn check_gene(&mut self, cx: &mut Context<Self>) {
        let typed = self.gene_input.read(cx).value().to_string();
        self.lookup_id += 1;
        let id = self.lookup_id;
        self.gene = GeneChecks {
            symbol: typed.trim().to_ascii_uppercase(),
            england: Lookup::Loading,
            australia: Lookup::Loading,
            g2p: Lookup::Loading,
            clingen: Lookup::Loading,
        };

        let (a, b, c, d) = (typed.clone(), typed.clone(), typed.clone(), typed);
        self._lookup_tasks = vec![
            Self::start_lookup(id, cx, move || panelapp::lookup_gene(Instance::England, &a), |g| &mut g.england),
            Self::start_lookup(id, cx, move || panelapp::lookup_gene(Instance::Australia, &b), |g| &mut g.australia),
            Self::start_lookup(id, cx, move || g2p::lookup_gene(&c), |g| &mut g.g2p),
            Self::start_lookup(id, cx, move || clingen::lookup_gene(&d), |g| &mut g.clingen),
        ];
        cx.notify();
    }

    /// Run one lookup in the background, then store its answer in `slot`.
    /// An answer that arrives after a newer check was started is dropped.
    fn start_lookup<T: Send + 'static>(
        id: u64,
        cx: &mut Context<Self>,
        work: impl FnOnce() -> Result<Fetched<T>, SourceError> + Send + 'static,
        slot: fn(&mut GeneChecks) -> &mut Lookup<T>,
    ) -> Task<()> {
        let pending = cx.background_spawn(async move { work() });
        cx.spawn(async move |this, cx| {
            let result = pending.await;
            this.update(cx, |this, cx| {
                if this.lookup_id != id {
                    return;
                }
                *slot(&mut this.gene) = match result {
                    Ok(found) => Lookup::Done(found),
                    Err(error) => Lookup::Failed(error.to_string()),
                };
                cx.notify();
            })
            .ok();
        })
    }

    fn current(&self) -> &Evidence {
        match self.editing {
            Editing::Today => &self.today,
            Editing::Record(i) => &self.records[i].evidence,
        }
    }

    fn current_mut(&mut self) -> &mut Evidence {
        match self.editing {
            Editing::Today => &mut self.today,
            Editing::Record(i) => &mut self.records[i].evidence,
        }
    }

    fn editing_label(&self) -> String {
        match self.editing {
            Editing::Today => "Today (you)".to_string(),
            Editing::Record(i) => format!("{} · {}", self.records[i].label, self.records[i].year),
        }
    }

    fn toggle(&mut self, criterion: &Criterion) {
        let evidence = self.current_mut();
        if evidence.remove(&criterion.code).is_none() {
            evidence.insert(criterion.code.clone(), criterion.strength);
        }
    }

    fn next_strength(&mut self, criterion: &Criterion) {
        let Some(current) = self.current().get(&criterion.code).copied() else { return };
        let options = criterion.allowed_strengths();
        let at = options.iter().position(|s| *s == current).unwrap_or(0);
        self.current_mut().insert(criterion.code.clone(), options[(at + 1) % options.len()]);
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        if mode == Mode::NewVariant {
            self.editing = Editing::Today;
        }
    }

    fn add_record(&mut self) {
        let label = format!("Record {}", self.records.len() + 1);
        self.records.push(Record { label, year: 2024, class: Class::Vus, evidence: Evidence::new() });
        self.editing = Editing::Record(self.records.len() - 1);
    }

    fn remove_record(&mut self, index: usize) {
        self.records.remove(index);
        self.editing = Editing::Today;
    }

    /// The fictional variant from the design sketch, so the history view can be tried at once.
    fn load_example(&mut self) {
        let evidence = |list: &[(&str, Strength)]| -> Evidence {
            list.iter().map(|(code, strength)| (code.to_string(), *strength)).collect()
        };
        self.records = vec![
            Record {
                label: "Lab A (example)".into(),
                year: 2019,
                class: Class::Vus,
                evidence: evidence(&[("PM2", Strength::Moderate), ("PP3", Strength::Supporting)]),
            },
            Record {
                label: "Lab B (example)".into(),
                year: 2022,
                class: Class::Vus,
                evidence: evidence(&[
                    ("PM2", Strength::Moderate),
                    ("PP3", Strength::Supporting),
                    ("PP4", Strength::Supporting),
                ]),
            },
        ];
        self.today = evidence(&[
            ("PM2", Strength::Supporting),
            ("PP3", Strength::Supporting),
            ("PP4", Strength::Supporting),
            ("PM5", Strength::Moderate),
        ]);
        self.editing = Editing::Today;
    }

    /// Copy the latest record's evidence into today's evidence, to adjust from there.
    fn start_from_latest(&mut self) {
        let latest = self.records.iter().enumerate().max_by_key(|(i, r)| (r.year, *i)).map(|(_, r)| r.evidence.clone());
        if let Some(evidence) = latest {
            self.today = evidence;
        }
    }

    // ---------- gene check ----------

    fn gene_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);

        let mut link = div()
            .flex()
            .items_center()
            .gap_2()
            .child(div().text_xs().text_color(muted).child("Does this gene cause the disease? (your decision)"));
        for (choice, label) in [
            (GeneLink::Established, "Yes, established"),
            (GeneLink::NotEstablished, "No, or doubtful"),
            (GeneLink::Unconfirmed, "Not decided"),
        ] {
            link = link.child(
                Button::new(SharedString::from(format!("gene-link-{label}")))
                    .label(label)
                    .small()
                    .selected(self.gene_link == choice)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.gene_link = choice;
                        cx.notify();
                    })),
            );
        }

        let g = &self.gene;
        let body: AnyElement = if matches!(g.clingen, Lookup::Idle) {
            div()
                .text_xs()
                .text_color(muted)
                .child("Type a gene symbol and press Enter or Check. ACMG/AMP criteria assume the gene is known to cause the disease.")
                .into_any_element()
        } else if !self.gene_cards_open {
            div().into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(self.source_card("ClinGen gene-disease validity", &g.clingen, describe_clingen, cx))
                        .child(self.source_card("Gene2Phenotype (G2P)", &g.g2p, describe_g2p, cx))
                        .child(self.source_card("PanelApp England", &g.england, describe_panelapp, cx))
                        .child(self.source_card("PanelApp Australia", &g.australia, describe_panelapp, cx)),
                )
                .child(div().text_xs().text_color(muted).child(
                    "Sources can disagree, and they are expert opinion about the gene, not about your variant. You decide whether this gene causes the disease.",
                ))
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).child("GENE"))
                    .child(div().w(px(220.)).child(Input::new(&self.gene_input)))
                    .child(Button::new("gene-check").label("Check gene").small().outline().on_click(cx.listener(
                        |this, _, _, cx| {
                            this.check_gene(cx);
                        },
                    )))
                    .when(!matches!(g.clingen, Lookup::Idle), |row| {
                        row.child(
                            Button::new("gene-cards-toggle")
                                .label(if self.gene_cards_open { "Hide results" } else { "Show results" })
                                .small()
                                .outline()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.gene_cards_open = !this.gene_cards_open;
                                    cx.notify();
                                })),
                        )
                    })
                    .child(link),
            )
            .child(body)
    }

    /// One source's answer. A failure is shown as a failure, never as "nothing found".
    fn source_card<T>(
        &self,
        title: &'static str,
        lookup: &Lookup<T>,
        describe: fn(&Fetched<T>) -> (String, Vec<String>),
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (border, muted, warning) = (cx.theme().border, cx.theme().muted_foreground, cx.theme().warning);
        let content: AnyElement = match lookup {
            Lookup::Idle => div().into_any_element(),
            Lookup::Loading => div().text_sm().text_color(muted).child("Checking…").into_any_element(),
            Lookup::Failed(message) => div()
                .text_sm()
                .text_color(warning)
                .child(format!("Could not check: {message}. This is not \"nothing found\"."))
                .into_any_element(),
            Lookup::Done(found) => {
                let (headline, lines) = describe(found);
                let mut col = div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().text_color(muted).child(format!("fetched {}", found.fetched_on)))
                    .child(div().text_sm().child(headline));
                for line in lines {
                    col = col.child(div().text_xs().text_color(muted).child(line));
                }
                col.into_any_element()
            }
        };
        div()
            .w(px(590.))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(border)
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(title))
            .child(content)
    }

    // ---------- criteria list (left) ----------

    fn criterion_row(&self, c: &Criterion, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let (border, muted) = (theme.border, theme.muted_foreground);
        let strength = self.current().get(&c.code).copied();
        let colour = direction_colour(c.direction);

        let toggle_target = c.clone();
        let cycle_target = c.clone();
        let source = self.rules.source(&c.source).map(|s| s.citation.clone());

        div()
            .flex()
            .flex_col()
            .gap_1()
            .py_2()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(5.)).h(px(22.)).rounded_sm().bg(colour).flex_none())
                    .child(
                        Button::new(SharedString::from(format!("apply-{}", c.code)))
                            .label(c.code.clone())
                            .small()
                            .selected(strength.is_some())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle(&toggle_target);
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1().min_w_0().text_sm().child(c.title.clone()))
                    .when(c.discouraged, |row| row.child(div().text_xs().text_color(muted).child("discouraged")))
                    .when_some(strength, |row, strength| {
                        row.child(
                            Button::new(SharedString::from(format!("strength-{}", c.code)))
                                .label(strength.label())
                                .small()
                                .outline()
                                .tooltip("Click to change the strength")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.next_strength(&cycle_target);
                                    cx.notify();
                                })),
                        )
                    }),
            )
            .when(self.learning, |col| {
                col.child(
                    div()
                        .pl(px(13.))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_xs()
                        .text_color(muted)
                        .child(c.summary.clone())
                        .when_some(c.note.clone(), |d, note| d.child(format!("Note: {note}")))
                        .when_some(source, |d, citation| d.child(format!("Source: {citation}"))),
                )
            })
    }

    fn criteria_column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let mut column = div().flex().flex_col().gap_4();
        for (direction, heading) in [
            (Direction::Pathogenic, "Evidence for disease (pathogenic)"),
            (Direction::Benign, "Evidence against disease (benign)"),
        ] {
            let mut group = div()
                .flex()
                .flex_col()
                .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).pb_1().child(heading));
            for category in Category::ALL {
                let in_category: Vec<&Criterion> =
                    self.rules.criteria.iter().filter(|c| c.direction == direction && c.category == category).collect();
                for c in in_category {
                    group = group.child(self.criterion_row(c, cx));
                }
            }
            column = column.child(group);
        }
        column
    }

    // ---------- records strip (reported variant) ----------

    fn records_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let mut chips = div().flex().flex_wrap().items_center().gap_2();
        for (i, r) in self.records.iter().enumerate() {
            chips = chips.child(
                Button::new(SharedString::from(format!("record-{i}")))
                    .label(format!("{} · {}", r.label, r.year))
                    .small()
                    .selected(self.editing == Editing::Record(i))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.editing = Editing::Record(i);
                        cx.notify();
                    })),
            );
        }
        chips = chips
            .child(
                Button::new("record-today")
                    .label("Today (you)")
                    .small()
                    .selected(self.editing == Editing::Today)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.editing = Editing::Today;
                        cx.notify();
                    })),
            )
            .child(Button::new("record-add").label("+ Add record").small().outline().on_click(cx.listener(
                |this, _, _, cx| {
                    this.add_record();
                    cx.notify();
                },
            )));
        if self.records.is_empty() {
            chips =
                chips.child(Button::new("record-example").label("Load fictional example").small().outline().on_click(
                    cx.listener(|this, _, _, cx| {
                        this.load_example();
                        cx.notify();
                    }),
                ));
        }

        let detail = match self.editing {
            Editing::Record(i) => {
                let record = &self.records[i];
                let mut row = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(muted).child("Class this record reported:"));
                for class in
                    [Class::Benign, Class::LikelyBenign, Class::Vus, Class::LikelyPathogenic, Class::Pathogenic]
                {
                    row = row.child(
                        Button::new(SharedString::from(format!("class-{i}-{}", class.short())))
                            .label(class.short())
                            .small()
                            .selected(record.class == class)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.records[i].class = class;
                                cx.notify();
                            })),
                    );
                }
                row.child(div().text_xs().text_color(muted).pl_2().child("Year:"))
                    .child(Button::new("year-down").label("−").small().outline().on_click(cx.listener(
                        move |this, _, _, cx| {
                            let year = &mut this.records[i].year;
                            *year = year.saturating_sub(1).max(2000);
                            cx.notify();
                        },
                    )))
                    .child(div().text_sm().child(record.year.to_string()))
                    .child(Button::new("year-up").label("+").small().outline().on_click(cx.listener(
                        move |this, _, _, cx| {
                            let year = &mut this.records[i].year;
                            *year = (*year + 1).min(2100);
                            cx.notify();
                        },
                    )))
                    .child(Button::new("record-remove").label("Remove record").small().outline().on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.remove_record(i);
                            cx.notify();
                        },
                    )))
                    .into_any_element()
            }
            Editing::Today => div()
                .flex()
                .items_center()
                .gap_2()
                .when(!self.records.is_empty(), |row| {
                    row.child(
                        Button::new("copy-latest")
                            .label("Start from the latest record's evidence")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_from_latest();
                                cx.notify();
                            })),
                    )
                })
                .child(
                    div()
                        .text_xs()
                        .text_color(muted)
                        .child("Editing your own evidence for today. Change, add or remove criteria to re-evaluate."),
                )
                .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child("RECORDS: what earlier labs reported and used. Choose one to edit its evidence."),
            )
            .child(chips)
            .child(detail)
    }

    // ---------- results (right) ----------

    fn evidence_chip(
        &self,
        code: &str,
        strength: Option<Strength>,
        changed: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (muted, foreground) = (cx.theme().muted_foreground, cx.theme().foreground);
        let cell = div().flex().items_center().gap_1().p_1().rounded_sm();
        let Some(strength) = strength else {
            return cell.text_color(muted).child("–");
        };
        let direction = self.rules.criterion(code).map_or(Direction::Pathogenic, |c| c.direction);
        let points = self.rules.magnitude(strength);
        let sign = if direction == Direction::Pathogenic { "+" } else { "−" };
        cell.when(changed, |d| d.border_2().border_color(foreground))
            .child(
                div()
                    .px_1()
                    .rounded_sm()
                    .bg(direction_colour(direction))
                    .text_color(rgb(0xffffff))
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .child(format!("{sign}{points}")),
            )
            .child(div().text_xs().child(strength.label()))
    }

    fn verdict_card(&self, re: &Reevaluation, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let latest = re.latest_index.map(|i| &re.records[i]);
        let mut card = div().flex().flex_col().gap_2().p_3().rounded_md().border_1().border_color(border);

        let (Some(verdict), Some(latest)) = (re.verdict, latest) else {
            let hint = if self.records.is_empty() {
                "Add the records (what earlier labs reported and used), then apply today's evidence to get a verdict."
            } else {
                "Apply today's evidence, or start from the latest record's evidence, to get a verdict."
            };
            return card.child(div().text_sm().text_color(muted).child(hint));
        };

        let (symbol, title) = match verdict {
            Verdict::Holds => ("=", "The classification holds"),
            Verdict::UpgradeCandidate => ("↑", "Upgrade candidate"),
            Verdict::DowngradeCandidate => ("↓", "Downgrade candidate"),
        };
        let today = re.today.points_class;
        card = card
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().text_xl().font_weight(FontWeight::BOLD).child(format!("{symbol}  {title}")))
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(class_fill(today))
                            .text_color(class_ink(today))
                            .text_sm()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("Today: {}", today.label())),
                    ),
            )
            .child(div().text_sm().child(format!(
                "The latest record ({}, {}) reported {}. Today's evidence gives {} points: {}.",
                latest.label,
                latest.year,
                latest.reported_class.label(),
                re.today.total_points,
                today.label()
            )))
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child("This is a candidate for you to review, not a decision. Check each change below."),
            );
        card
    }

    fn reasons_list(&self, re: &Reevaluation, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        if re.reasons.is_empty() {
            return list.child(div().text_color(muted).child("No differences from the latest record."));
        }
        for reason in &re.reasons {
            let text = match reason {
                Reason::Added { code, strength } => {
                    format!("{code} added at {}: new evidence since the latest record.", strength.label())
                }
                Reason::Removed { code } => format!("{code} is no longer applied."),
                Reason::StrengthChanged { code, from, to } => {
                    format!("{code} changed from {} to {}.", from.label(), to.label())
                }
                Reason::RuleChanged { code, from, to, source } => {
                    let citation = self.rules.source(source).map_or(source.as_str(), |s| s.citation.as_str());
                    format!(
                        "{code}: the record used {} (the strength in the 2015 paper). The current rule is {}. Source: {citation}",
                        from.label(),
                        to.label()
                    )
                }
            };
            list = list.child(div().flex().gap_2().child("•").child(div().flex_1().min_w_0().child(text)));
        }
        list
    }

    fn history_table(&self, re: &Reevaluation, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let cell = |content: AnyElement| div().flex_1().min_w(px(120.)).px_1().child(content);
        let first = |content: AnyElement| div().w(px(56.)).flex_none().child(content);
        let latest_cells = re.latest_index;

        let mut header = div().flex().items_start().py_1().child(first(div().into_any_element()));
        for r in &re.records {
            header = header.child(cell(
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(format!("{} · {}", r.label, r.year)))
                    .child(div().text_xs().text_color(muted).child(format!("reported {}", r.reported_class.short())))
                    .into_any_element(),
            ));
        }
        header = header
            .child(cell(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("Today (you)").into_any_element()));

        let mut table = div().flex().flex_col().child(header);
        for row in &re.rows {
            let mut line = div().flex().items_center().py_1().border_t_1().border_color(border).child(first(
                div().text_sm().font_weight(FontWeight::SEMIBOLD).child(row.code.clone()).into_any_element(),
            ));
            let today_index = row.cells.len() - 1;
            for (i, strength) in row.cells.iter().enumerate() {
                // Outline today's cell when it differs from the latest record.
                let changed = i == today_index && latest_cells.is_some_and(|l| row.cells[l] != *strength);
                line = line.child(cell(self.evidence_chip(&row.code, *strength, changed, cx).into_any_element()));
            }
            table = table.child(line);
        }

        let mut total = div()
            .flex()
            .items_start()
            .py_1()
            .border_t_1()
            .border_color(border)
            .child(first(div().text_xs().font_weight(FontWeight::SEMIBOLD).child("Total").into_any_element()));
        for r in &re.records {
            let a = &r.assessment;
            let mut col = div().flex().flex_col().child(div().text_sm().child(format!(
                "{} pts → {}",
                a.total_points,
                a.points_class.short()
            )));
            if !r.reported_matches_rescore() {
                col = col.child(div().text_xs().text_color(muted).child(format!(
                    "! it reported {}; its evidence scores differently today",
                    r.reported_class.short()
                )));
            }
            total = total.child(cell(col.into_any_element()));
        }
        total = total.child(cell(
            div()
                .text_sm()
                .font_weight(FontWeight::BOLD)
                .child(format!("{} pts → {}", re.today.total_points, re.today.points_class.short()))
                .into_any_element(),
        ));
        table.child(total)
    }

    fn points_scale(&self, total: i32, active: Class, cx: &mut Context<Self>) -> impl IntoElement {
        let (foreground, muted) = (cx.theme().foreground, cx.theme().muted_foreground);
        // (label, first point, first point of the next zone, class)
        let zones = [
            ("B", SCALE_MIN, -6, Class::Benign),
            ("Likely benign", -6, 0, Class::LikelyBenign),
            ("VUS", 0, 6, Class::Vus),
            ("Likely pathogenic", 6, 10, Class::LikelyPathogenic),
            ("P", 10, SCALE_MAX, Class::Pathogenic),
        ];
        let x_of = |points: i32| (points - SCALE_MIN) as f32 * SCALE_UNIT;
        let marker_x = x_of(total.clamp(SCALE_MIN, SCALE_MAX - 1)) + SCALE_UNIT / 2.0 - 7.0;

        let mut bar = div().flex().absolute().top(px(18.)).left(px(0.));
        for (label, from, to, class) in zones {
            let is_active = class == active;
            let fill = class_fill(class);
            bar = bar.child(
                div()
                    .w(px((to - from) as f32 * SCALE_UNIT))
                    .h(px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .bg(fill)
                    .text_color(class_ink(class))
                    .text_xs()
                    .font_weight(if is_active { FontWeight::BOLD } else { FontWeight::NORMAL })
                    .border_2()
                    .border_color(if is_active { foreground } else { fill.into() })
                    .child(label),
            );
        }

        let mut ticks = div().absolute().top(px(54.)).left(px(0.)).w(px(600.));
        for edge in [-6, 0, 6, 10] {
            ticks = ticks.child(
                div()
                    .absolute()
                    .left(px(x_of(edge) - 10.))
                    .w(px(20.))
                    .flex()
                    .justify_center()
                    .text_xs()
                    .text_color(muted)
                    .child(edge.to_string()),
            );
        }

        div()
            .relative()
            .w(px((SCALE_MAX - SCALE_MIN) as f32 * SCALE_UNIT))
            .h(px(74.))
            .child(div().absolute().top(px(0.)).left(px(marker_x)).text_color(foreground).child("▼"))
            .child(bar)
            .child(ticks)
    }

    fn evidence_lanes(&self, result: &Assessment, cx: &mut Context<Self>) -> impl IntoElement {
        let (border, muted) = (cx.theme().border, cx.theme().muted_foreground);
        let mut lanes = div().flex().flex_col();
        for category in Category::ALL {
            let items: Vec<_> = result
                .items
                .iter()
                .filter(|i| self.rules.criterion(&i.code).is_some_and(|c| c.category == category))
                .collect();
            let mut track = div().flex().flex_wrap().items_center().gap_1().min_h(px(30.)).flex_1();
            if items.is_empty() {
                track = track.child(div().text_xs().text_color(muted).child("none applied"));
            }
            for item in items {
                let width = (item.points.unsigned_abs() as f32 * POINT_WIDTH).max(POINT_WIDTH);
                let sign = if item.points > 0 { "+" } else { "−" };
                track = track
                    .child(
                        div()
                            .w(px(width))
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_r_md()
                            .bg(direction_colour(item.direction))
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child(item.code.clone()),
                    )
                    .child(div().text_xs().text_color(muted).pr_2().child(format!(
                        "{sign}{} {}",
                        item.points.unsigned_abs(),
                        item.strength.label()
                    )));
            }
            lanes = lanes.child(
                div()
                    .flex()
                    .items_center()
                    .py_1()
                    .border_t_1()
                    .border_color(border)
                    .child(div().w(px(120.)).flex_none().text_xs().child(category.label()))
                    .child(track),
            );
        }
        lanes
    }

    fn what_changes(&self, result: &Assessment, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        if let Some(next) = result.to_upgrade {
            list = list.child(format!(
                "Up: {} more point{} of pathogenic evidence reaches {}.",
                next.points_needed,
                plural(next.points_needed),
                next.class.label()
            ));
        }
        if let Some(next) = result.to_downgrade {
            list = list.child(format!(
                "Down: {} more point{} of benign evidence reaches {}.",
                next.points_needed,
                plural(next.points_needed),
                next.class.label()
            ));
        }
        list.child(
            div()
                .text_xs()
                .text_color(muted)
                .child("To find the evidence that could move it, look at the lanes with nothing applied."),
        )
    }

    fn warnings(&self, result: &Assessment, cx: &mut Context<Self>) -> impl IntoElement {
        let warning = cx.theme().warning;
        let mut list = div().flex().flex_col().gap_1().text_sm();
        let mut texts: Vec<String> = Vec::new();
        match self.gene_link {
            GeneLink::Established => {}
            GeneLink::Unconfirmed => texts.push(
                "You have not confirmed that this gene causes the disease. ACMG/AMP criteria assume it does.".to_string(),
            ),
            GeneLink::NotEstablished => texts.push(
                "You marked the gene-disease link as doubtful. ACMG/AMP criteria do not apply without it; treat any class below with great care.".to_string(),
            ),
        }
        texts.extend(result.warnings.iter().map(|w| match w {
            Warning::ConflictingEvidence => {
                "Evidence points both ways. Check that each criterion really applies.".to_string()
            }
            Warning::ExclusivePair { a, b, reason } => format!("{a} and {b} together: {reason}"),
            Warning::Discouraged { code, note } => format!(
                "{code} is no longer recommended by the experts.{}",
                note.as_deref().map(|n| format!(" {n}")).unwrap_or_default()
            ),
            Warning::StrengthChanged { code, from, to } => {
                format!("{code} strength changed from {} to {}.", from.label(), to.label())
            }
        }));
        if texts.is_empty() {
            return list.child("No warnings.");
        }
        for text in texts {
            list = list.child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().text_color(warning).child("!"))
                    .child(div().flex_1().min_w_0().child(text)),
            );
        }
        list
    }
}

impl Render for Workbench {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rules = self.rules;
        // The screen only offers valid criteria and strengths, so these cannot fail.
        let result = rules.assess(&to_applied(self.current())).expect("the screen only offers valid criteria");
        let reevaluation = (self.mode == Mode::ReportedVariant).then(|| {
            let submissions: Vec<Submission> = self
                .records
                .iter()
                .map(|r| Submission {
                    label: r.label.clone(),
                    year: r.year,
                    reported_class: r.class,
                    reported_tier: None,
                    criteria: to_applied(&r.evidence),
                })
                .collect();
            rules.reevaluate(&submissions, &to_applied(&self.today)).expect("the screen only offers valid criteria")
        });

        let theme = cx.theme();
        let (background, foreground, border, muted, warning) =
            (theme.background, theme.foreground, theme.border, theme.muted_foreground, theme.warning);

        let classic = match (result.classic_class, result.frameworks_agree()) {
            (Some(class), Some(true)) => format!("2015 combining table: {} (same as the points).", class.label()),
            (Some(class), _) => format!(
                "2015 combining table: {}. It differs from the points; the two methods are not identical.",
                class.label()
            ),
            (None, _) => "2015 combining table: not defined for the strengths used.".to_string(),
        };

        let sign = if result.total_points > 0 { "+" } else { "" };
        let draft_note = format!(
            "{} · version {} · {} · research and learning use only",
            rules.meta.name,
            rules.meta.version,
            if rules.meta.status == classivar_core::rules::ReviewStatus::Unverified {
                "UNVERIFIED rules"
            } else {
                "reviewed rules"
            }
        );

        let section = |title: &'static str| {
            div().text_xs().font_weight(FontWeight::SEMIBOLD).text_color(muted).pb_2().child(title)
        };

        let mut right =
            div().id("result").flex_1().min_w_0().h_full().overflow_y_scroll().p_4().flex().flex_col().gap_5();

        if let Some(re) = &reevaluation {
            right = right
                .child(div().child(section("VERDICT")).child(self.verdict_card(re, cx)))
                .when(re.verdict.is_some(), |d| {
                    d.child(
                        div().child(section("WHAT CHANGED SINCE THE LATEST RECORD")).child(self.reasons_list(re, cx)),
                    )
                })
                .when(!re.records.is_empty(), |d| {
                    d.child(div().child(section("HISTORY AGAINST TODAY'S RULES")).child(self.history_table(re, cx)))
                })
                .child(
                    div()
                        .pt_2()
                        .border_t_1()
                        .border_color(border)
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(format!("Details for: {}", self.editing_label())),
                );
        }

        right = right
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(class_fill(result.points_class))
                            .text_color(class_ink(result.points_class))
                            .font_weight(FontWeight::BOLD)
                            .child(result.points_class.label()),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("{sign}{} points", result.total_points)),
                    )
                    .when_some(result.vus_tier.clone(), |row, tier| row.child(tier)),
            )
            .child(div().text_sm().text_color(muted).child(classic))
            .child(div().child(section("WHERE IT SITS")).child(self.points_scale(
                result.total_points,
                result.points_class,
                cx,
            )))
            .child(div().child(section("WHAT IT IS MADE OF")).child(self.evidence_lanes(&result, cx)))
            .child(div().child(section("WHAT WOULD CHANGE IT")).child(self.what_changes(&result, cx)))
            .child(div().child(section("WARNINGS")).child(self.warnings(&result, cx)));

        let intro = match (self.mode, self.editing) {
            (Mode::NewVariant, _) => {
                "New variant. Click a code to apply it. You decide which criteria apply; the app adds up the points."
                    .to_string()
            }
            (Mode::ReportedVariant, _) => {
                format!("Editing: {}. Click a code to apply it or remove it.", self.editing_label())
            }
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(background)
            .text_color(foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(div().text_lg().font_weight(FontWeight::BOLD).child("ClassiVAR"))
                            .child(div().text_xs().text_color(warning).child(draft_note)),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("mode-new")
                                    .label("New variant")
                                    .small()
                                    .selected(self.mode == Mode::NewVariant)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_mode(Mode::NewVariant);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("mode-reported")
                                    .label("Reported variant")
                                    .small()
                                    .selected(self.mode == Mode::ReportedVariant)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_mode(Mode::ReportedVariant);
                                        cx.notify();
                                    })),
                            )
                            .child(div().w(px(16.)))
                            .child(
                                Button::new("learning")
                                    .label(if self.learning { "Learning: on" } else { "Learning: off" })
                                    .small()
                                    .selected(self.learning)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.learning = !this.learning;
                                        cx.notify();
                                    })),
                            )
                            .child(Button::new("clear").label("Clear evidence").small().outline().on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.current_mut().clear();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(self.gene_strip(cx))
            .when(self.mode == Mode::ReportedVariant, |d| d.child(self.records_strip(cx)))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("criteria")
                            .w(px(560.))
                            .h_full()
                            .flex_none()
                            .overflow_y_scroll()
                            .p_4()
                            .border_r_1()
                            .border_color(border)
                            .child(div().pb_3().text_sm().text_color(muted).child(intro))
                            .child(self.criteria_column(cx)),
                    )
                    .child(right),
            )
    }
}
