//! The earlier records of a case, and the mode of the screen.

use super::*;

impl Workbench {
    pub(super) fn set_mode(&mut self, mode: Mode) {
        self.welcome = false;
        self.category = None;
        self.pointed_zone = None;
        self.close_pages();
        self.baseline = None;
        self.reading = false;
        self.comparison = false;
        self.inspected = None;
        self.hovered = None;
        self.preview = false;
        self.mode = mode;
        if mode == Mode::NewVariant {
            self.editing = Editing::Today;
        }
    }

    /// Open an earlier record for editing and show its name in the name box.
    pub(super) fn show_record(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.records.len() {
            return;
        }
        self.close_pages();
        let record = &self.records[index];
        let name = record.label.clone();
        self.inspected = None;
        self.preview = false;
        self.editing = Editing::Record(index);
        let condition = record.condition.as_ref().map(|c| c.name.clone()).unwrap_or_default();
        self.boxes.record_name.update(cx, |input, cx| input.set_value(name, window, cx));
        self.variant.record_condition_input.update(cx, |input, cx| input.set_value(condition, window, cx));
    }

    pub(super) fn add_record(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.records.len() >= MAX_RECORDS {
            self.file_status = Some(Status::attention(format!("A case holds at most {MAX_RECORDS} earlier records.")));
            return;
        }
        let label = format!("Record {}", self.next_record);
        self.next_record += 1;
        self.records.push(Record {
            label,
            year: classivar_sources::current_year(),
            class: None,
            tier: None,
            evidence: Evidence::new(),
            condition: None,
            source: None,
        });
        self.show_record(self.records.len() - 1, window, cx);
    }

    pub(super) fn remove_record(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.records.len() {
            return;
        }
        self.category = None;
        self.baseline = None;
        self.records.remove(index);
        // Stay next to the record that took its place, rather than jumping away.
        if self.records.is_empty() {
            self.editing = Editing::Today;
        } else {
            self.show_record(index.min(self.records.len() - 1), window, cx);
        }
        self.confirm = None;
    }

    /// The next "Record N" number, continuing after the highest one in `records`.
    pub(super) fn next_record_number(records: &[Record]) -> u32 {
        records
            .iter()
            .filter_map(|record| record.label.strip_prefix("Record ").and_then(|number| number.parse::<u32>().ok()))
            .max()
            .unwrap_or(0)
            + 1
    }
}
