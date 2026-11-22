//! The checked variant, drawn as one clear block: who checked it and when, the
//! name in the code face with its gene, the protein change, both builds, and the
//! ids as chips. The words come from `identity_view`, a plain tested function.

use classivar_core::variant::VariantIdentity;

use super::*;

/// What the card shows, in order. Only what the registry gave.
#[derive(Debug, PartialEq)]
pub(super) struct IdentityView {
    /// What kind of name leads the card: "MANE Select", or the genomic build.
    pub(super) heading: &'static str,
    pub(super) name: Option<String>,
    pub(super) gene: Option<String>,
    pub(super) protein: Option<String>,
    /// The genomic names not already used as the lead name.
    pub(super) builds: Vec<(&'static str, String)>,
    pub(super) ids: Vec<String>,
    /// The service and the date, for example "ClinGen Allele Registry · 2026-11-22".
    pub(super) checked: String,
}

pub(super) fn identity_view(identity: &VariantIdentity) -> IdentityView {
    let (heading, name) = match (&identity.mane_select, &identity.grch38, &identity.grch37) {
        (Some(mane), _, _) => ("MANE Select", Some(mane.clone())),
        (None, Some(g38), _) => ("Genomic, GRCh38", Some(g38.clone())),
        (None, None, Some(g37)) => ("Genomic, GRCh37", Some(g37.clone())),
        (None, None, None) => ("Variant", None),
    };
    let mut builds = Vec::new();
    for (label, value) in [("GRCh38", &identity.grch38), ("GRCh37", &identity.grch37)] {
        if let Some(value) = value.as_ref().filter(|value| Some(*value) != name.as_ref()) {
            builds.push((label, value.clone()));
        }
    }
    let mut ids = Vec::new();
    ids.extend(identity.allele_id.clone());
    ids.extend(identity.clinvar_variation.map(|id| format!("VCV{id:09}")));
    ids.extend(identity.dbsnp.map(|rs| format!("rs{rs}")));
    if identity.allele_id.is_none() {
        ids.push("not yet registered (no CA id)".into());
    }
    IdentityView {
        heading,
        name,
        gene: identity.gene.clone(),
        protein: identity.protein.clone(),
        builds,
        ids,
        checked: format!("{} · {}", identity.checked_with, identity.checked_on),
    }
}

impl Workbench {
    /// One identity as a card. `changed`: the variant box no longer matches this check.
    pub(super) fn identity_card(&self, identity: &VariantIdentity, changed: bool, cx: &mut Context<Self>) -> Div {
        let (muted, border) = (cx.theme().muted_foreground, cx.theme().border);
        let view = identity_view(identity);
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(crate::look::eyebrow(format!("✓ Checked · {}", view.heading), cx))
            .children(view.gene.clone().map(|gene| gene_badge(gene, cx)));
        let builds = view.builds.iter().map(|(label, value)| {
            div()
                .flex()
                .gap_2()
                .child(div().w(px(52.)).flex_none().text_color(muted).child(*label))
                .child(div().min_w_0().font_family(crate::look::MONO).child(value.clone()))
        });
        let ids = div().flex().flex_wrap().gap_1().children(view.ids.iter().map(|id| {
            div()
                .px(px(6.))
                .py(px(1.))
                .rounded(px(5.))
                .border_1()
                .border_color(border)
                .font_family(crate::look::MONO)
                .child(id.clone())
        }));
        crate::look::card(cx)
            .flex()
            .flex_col()
            .gap_2()
            .text_xs()
            .child(head)
            .children(view.name.map(|name| crate::look::figure(14.).line_height(px(20.)).child(name)))
            .children(view.protein.map(|p| div().font_family(crate::look::MONO).text_color(muted).child(p)))
            .child(div().flex().flex_col().gap_0p5().children(builds))
            .child(ids)
            .child(caption(muted).child(view.checked))
            .when(changed, |d| {
                d.child(notice(
                    "The variant box changed since this check. Check again to update it.",
                    warning_fill(),
                    warning_ink(),
                ))
            })
    }
}

/// The gene as a small solid label beside the heading.
fn gene_badge(gene: String, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .px(px(7.))
        .py(px(1.))
        .rounded(px(5.))
        .bg(theme.foreground)
        .text_color(theme.background)
        .font_family(crate::look::MONO)
        .font_weight(FontWeight::SEMIBOLD)
        .child(gene)
}
