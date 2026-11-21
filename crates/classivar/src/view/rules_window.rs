//! The Rules button: check another rule file (an expert panel's gene-specific
//! rules, for example) and open it in a new window. The case on screen is not
//! touched; open it in the new window to see it under those rules.

use super::*;

impl Workbench {
    pub(super) fn open_rules(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open a rule file in a new window".into()),
        });
        self._file_task = Some(cx.spawn_in(window, async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Err(error)) => {
                    let message = Status::attention(format!("Could not open the file dialog: {error}"));
                    this.update(cx, |this, cx| this.set_file_status(message, cx)).ok();
                    return;
                }
                _ => None,
            };
            let Some(path) = path else { return };
            let loaded = cx.background_spawn(async move { crate::rule_file::read_rule_file(&path) }).await;
            this.update(cx, |this, cx| this.finish_open_rules(loaded, cx)).ok();
        }));
    }

    fn finish_open_rules(&mut self, loaded: Result<&'static RuleSet, String>, cx: &mut Context<Self>) {
        let message = match loaded {
            Ok(rules) => {
                let options = crate::window_options(cx);
                match open_window(options, cx, |window, cx| cx.new(|cx| Workbench::new(rules, window, cx))) {
                    Ok(_) => Status::done(format!(
                        "Opened {} ({}) in a new window. This window keeps its rules; open the case there to compare.",
                        rules.meta.id,
                        rules_tag(rules)
                    )),
                    Err(error) => {
                        Status::attention(format!("The rules are valid, but a window could not open: {error}"))
                    }
                }
            }
            Err(error) => Status::attention(format!("Could not use that rule file: {error}")),
        };
        self.set_file_status(message, cx);
    }
}
