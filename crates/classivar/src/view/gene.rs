//! Checking a gene against the four sources.

use super::*;

impl Workbench {
    /// Ask all four gene sources about the gene in the text box. A different gene
    /// from the one this case is about starts a new case, so the curator is asked first.
    pub(super) fn check_gene(&mut self, cx: &mut Context<Self>) {
        let typed = self.boxes.gene.read(cx).value().to_string();
        let symbol = typed.trim().to_ascii_uppercase();
        if !classivar_sources::is_gene_symbol(&symbol) {
            self.gene_error = Some(
                "Type a gene symbol before checking: letters, digits or hyphens, up to 20 characters.".to_string(),
            );
            cx.notify();
            return;
        }
        self.gene_error = None;
        if !self.case_gene.is_empty() && symbol != self.case_gene && self.has_case_data(cx) {
            self.ask(Confirm::NewGene(symbol));
            cx.notify();
            return;
        }
        self.run_gene_check(typed, cx);
    }

    /// Start the four lookups. Each network call runs in the background and fills
    /// in its own card when it arrives, so one slow or failing source does not
    /// hold up the others.
    pub(super) fn run_gene_check(&mut self, typed: String, cx: &mut Context<Self>) {
        self.gene_error = None;
        let symbol = typed.trim().to_ascii_uppercase();
        if symbol != self.case_gene {
            // The decision about the old gene says nothing about this one.
            self.gene_link = GeneDecision::NotDecided;
            self.case_gene = symbol;
        }
        self.lookup_id += 1;
        let id = self.lookup_id;
        self.gene = GeneChecks {
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
    pub(super) fn start_lookup<T: Send + 'static>(
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
}
